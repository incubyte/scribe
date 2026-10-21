type RecordBody = Record<string, unknown>;
type SnapshotMetadata = { fromCache: boolean; hasPendingWrites: boolean };
type Unsubscribe = () => void;
type DbError = { code: string; message: string };

export type FakeDocumentSnapshot = {
  id: string;
  exists: boolean;
  data(): RecordBody | undefined;
  metadata: SnapshotMetadata;
};

export type FakeQuerySnapshot = {
  docs: FakeDocumentSnapshot[];
  size: number;
  empty: boolean;
  metadata: SnapshotMetadata;
};

export type FakeDocumentReference = {
  id: string;
  path: string;
  update(data: RecordBody): Promise<void>;
  onSnapshot(next: (snapshot: FakeDocumentSnapshot) => void): Unsubscribe;
};

export type FakeCollectionReference = {
  path: string;
  doc(id: string): FakeDocumentReference;
  onSnapshot(next: (snapshot: FakeQuerySnapshot) => void): Unsubscribe;
};

export type FakeDb = {
  collection(path: string): FakeCollectionReference;
  doc(path: string): FakeDocumentReference;
};

export type FakeDatabase = {
  seed(collection: string, id: string, data: RecordBody): void;
  write(collection: string, id: string, data: RecordBody): void;
  read(collection: string, id: string): RecordBody | undefined;
  failNextUpdate(code: string): void;
  db: FakeDb;
};

export type FakeClaudeRuntime = {
  claude: { use(name: string): Promise<unknown> };
  database: FakeDatabase;
  isIdle(): boolean;
};

type RuntimeOptions = {
  database?: "available" | "unavailable";
  canWriteData?: boolean | null;
  askClaude?: "available" | "unavailable";
  answer?: string;
};

type Scheduler = {
  schedule(task: () => void): void;
  later<T>(produce: () => T | Promise<T>): Promise<T>;
  isIdle(): boolean;
};

type Subscription = { collection: string; documentId?: string; deliver(): void };

const DEFAULT_ANSWER = "Here is what Claude drafted.";
const CONFIRMED: SnapshotMetadata = { fromCache: false, hasPendingWrites: false };

export function createFakeClaudeRuntime({
  database: databaseAvailability = "available",
  canWriteData = true,
  askClaude = "available",
  answer = DEFAULT_ANSWER,
}: RuntimeOptions = {}): FakeClaudeRuntime {
  const scheduler = createScheduler();
  const database = new InMemoryDatabase(scheduler);
  const namespaces = new Map<string, unknown>([["user", fakeUser(scheduler, canWriteData)]]);
  if (databaseAvailability === "available") {
    namespaces.set("db", database.db);
  }
  if (askClaude === "available") {
    namespaces.set("sample", fakeSample(scheduler, answer));
  }
  return {
    claude: { use: (name) => scheduler.later(() => namespaces.get(name) ?? null) },
    database,
    isIdle: () => scheduler.isIdle(),
  };
}

function createScheduler(): Scheduler {
  let pendingTasks = 0;
  const schedule = (task: () => void) => {
    pendingTasks += 1;
    setTimeout(() => {
      pendingTasks -= 1;
      task();
    }, 0);
  };
  return {
    schedule,
    later: <T>(produce: () => T | Promise<T>) => new Promise<T>((resolve) => schedule(() => resolve(produce()))),
    isIdle: () => pendingTasks === 0,
  };
}

function fakeUser(scheduler: Scheduler, canWriteData: boolean | null) {
  return {
    can: (capability: string) => scheduler.later(() => (capability === "data.write" ? canWriteData : null)),
  };
}

function fakeSample(scheduler: Scheduler, answer: string) {
  return () => scheduler.later(() => ({ text: answer, truncated: false }));
}

class InMemoryDatabase implements FakeDatabase {
  readonly db: FakeDb;
  private readonly scheduler: Scheduler;
  private readonly collections = new Map<string, Map<string, RecordBody>>();
  private readonly subscriptions = new Set<Subscription>();
  private readonly pendingFailures: string[] = [];

  constructor(scheduler: Scheduler) {
    this.scheduler = scheduler;
    this.db = createFakeDb(this);
  }

  seed(collection: string, id: string, data: RecordBody): void {
    this.documentsOf(collection).set(id, structuredClone(data));
  }

  write(collection: string, id: string, data: RecordBody): void {
    this.seed(collection, id, data);
    this.notify(collection, id);
  }

  read(collection: string, id: string): RecordBody | undefined {
    return this.collections.get(collection)?.get(id);
  }

  failNextUpdate(code: string): void {
    this.pendingFailures.push(code);
  }

  update(collection: string, id: string, patch: RecordBody): Promise<void> {
    return this.scheduler.later(() => this.applyUpdate(collection, id, patch));
  }

  watchDocument(collection: string, id: string, next: (snapshot: FakeDocumentSnapshot) => void): Unsubscribe {
    return this.subscribe({
      collection,
      documentId: id,
      deliver: () => next(documentSnapshot(id, this.read(collection, id))),
    });
  }

  watchCollection(collection: string, next: (snapshot: FakeQuerySnapshot) => void): Unsubscribe {
    return this.subscribe({ collection, deliver: () => next(querySnapshot(this.entriesInIdOrder(collection))) });
  }

  private applyUpdate(collection: string, id: string, patch: RecordBody): Promise<void> {
    const existing = this.read(collection, id);
    const failure = this.pendingFailures.shift();
    if (failure !== undefined) {
      return Promise.reject(dbError(failure, "The update was refused."));
    }
    if (existing === undefined) {
      return Promise.reject(dbError("invalid_argument", "update needs an existing document: " + collection + "/" + id));
    }
    this.write(collection, id, mergeBody(existing, patch));
    return Promise.resolve();
  }

  private subscribe(subscription: Subscription): Unsubscribe {
    this.subscriptions.add(subscription);
    this.deliverLater(subscription);
    return () => {
      this.subscriptions.delete(subscription);
    };
  }

  private notify(collection: string, id: string): void {
    [...this.subscriptions]
      .filter((subscription) => watches(subscription, collection, id))
      .forEach((subscription) => this.deliverLater(subscription));
  }

  private deliverLater(subscription: Subscription): void {
    this.scheduler.schedule(() => {
      if (this.subscriptions.has(subscription)) {
        subscription.deliver();
      }
    });
  }

  private documentsOf(collection: string): Map<string, RecordBody> {
    const documents = this.collections.get(collection) ?? new Map<string, RecordBody>();
    this.collections.set(collection, documents);
    return documents;
  }

  private entriesInIdOrder(collection: string): [string, RecordBody][] {
    return [...this.documentsOf(collection).entries()].sort(([leftId], [rightId]) => (leftId < rightId ? -1 : 1));
  }
}

function createFakeDb(database: InMemoryDatabase): FakeDb {
  return {
    collection: (path) => collectionReference(database, collectionPath(path)),
    doc: (path) => documentReference(database, ...documentPath(path)),
  };
}

function collectionReference(database: InMemoryDatabase, collection: string): FakeCollectionReference {
  return {
    path: collection,
    doc: (id) => documentReference(database, collection, id),
    onSnapshot: (next) => database.watchCollection(collection, next),
  };
}

function documentReference(database: InMemoryDatabase, collection: string, id: string): FakeDocumentReference {
  return {
    id,
    path: collection + "/" + id,
    update: (data) => database.update(collection, id, data),
    onSnapshot: (next) => database.watchDocument(collection, id, next),
  };
}

function collectionPath(path: string): string {
  if (path === "" || path.includes("/")) {
    throw new TypeError("The fake database holds top-level collections only, not " + path);
  }
  return path;
}

function documentPath(path: string): [string, string] {
  const [collection, id, ...nested] = path.split("/");
  if (!collection || !id || nested.length > 0) {
    throw new TypeError("A document path is <collection>/<id>, not " + path);
  }
  return [collection, id];
}

function watches(subscription: Subscription, collection: string, id: string): boolean {
  return subscription.collection === collection && (subscription.documentId ?? id) === id;
}

function documentSnapshot(id: string, body: RecordBody | undefined): FakeDocumentSnapshot {
  const frozenBody = body === undefined ? undefined : Object.freeze(structuredClone(body));
  return { id, exists: frozenBody !== undefined, data: () => frozenBody, metadata: CONFIRMED };
}

function querySnapshot(entries: [string, RecordBody][]): FakeQuerySnapshot {
  const docs = entries.map(([id, body]) => documentSnapshot(id, body));
  return { docs, size: docs.length, empty: docs.length === 0, metadata: CONFIRMED };
}

function dbError(code: string, message: string): DbError {
  return { code, message };
}

function isPlainObject(value: unknown): value is RecordBody {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function mergeBody(existing: RecordBody, patch: RecordBody): RecordBody {
  const merged = { ...existing };
  Object.entries(patch).forEach(([key, value]) => {
    const current = merged[key];
    merged[key] = isPlainObject(current) && isPlainObject(value) ? mergeBody(current, value) : value;
  });
  return merged;
}
