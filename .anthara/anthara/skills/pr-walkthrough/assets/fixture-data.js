// Fixture data for template.html — the template's test harness (spec 004, AC 5.6.9).
// Shape is normative: the skill emits walkthrough.json in this structure and wraps it
// as `window.WALKTHROUGH = {...};` in walkthrough-data.js. All prose fields are plain
// text with two inline forms the template renders: **bold** and `code`. Code `t` values
// are raw text; marks reference a line number and a literal span within that line — the
// template wraps the first occurrence at render time (AC 5.6.8). This fixture is the
// PR #212 scenario from docs/mockups/BRIEF.md, expressed in schema v2.
window.WALKTHROUGH = {
  schema_version: 2,
  generated_at: "2026-07-28T14:47:03Z",
  pr: {
    repo: "acme/gateway-api",
    number: 212,
    title: "Lock accounts after repeated failed logins",
    branch: "feat/auth-lockout",
    author: "mira",
    ticket: "GW-731",
    head_sha: "4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1",
    files: 34,
    hunks: 110,
    additions: 1284,
    deletions: 312,
    permalink_base: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1"
  },
  ac_source: "ticket:GW-731",
  acceptance_criteria: [
    {
      id: "AC-6", text: "Ops can unlock an account via admin endpoint",
      state: "uncovered", evidence: "no code implements it",
      implemented_by: [], verified_by: null
    },
    {
      id: "AC-4", text: "Audit events record the actor's IP",
      state: "untested", evidence: "`meta.ip` is written, no test asserts it",
      implemented_by: ["art2"], verified_by: null
    },
    {
      id: "AC-1", text: "Account locks after 5 failed attempts within 15 min",
      state: "covered", evidence: "\"locks on the fifth consecutive failure\" · CI passed",
      implemented_by: ["art1"],
      verified_by: { test_name: "locks on the fifth consecutive failure", file: "test/lockout.service.spec.ts", ci: { status: "passed", check: "unit" } }
    },
    {
      id: "AC-2", text: "Locked account answers 423 with Retry-After",
      state: "covered", evidence: "login.flow.e2e \"locked account gets 423\" · CI passed",
      implemented_by: ["art1"],
      verified_by: { test_name: "locked account gets 423", file: "test/login.flow.e2e.ts", ci: { status: "passed", check: "e2e" } }
    },
    {
      id: "AC-3", text: "Successful login resets the failure counter",
      state: "covered", evidence: "\"reset on success\" · CI passed",
      implemented_by: ["art1"],
      verified_by: { test_name: "reset on success", file: "test/login.flow.e2e.ts", ci: { status: "passed", check: "e2e" } }
    },
    {
      id: "AC-5", text: "Service accounts are exempt from lockout",
      state: "covered", evidence: "\"service accounts never lock\" · CI passed",
      implemented_by: ["art3"],
      verified_by: { test_name: "service actors bypass the limiter", file: "test/rate-limit.spec.ts", ci: { status: "passed", check: "unit" } }
    }
  ],
  packs: [
    { cite: "HIPAA 164.312(b)", text: "audit controls — **actor IP written but never asserted**", state: "marked", ref: "art2" },
    { cite: "OWASP A07", text: "identification & auth — lockout boundary", state: "marked", ref: "art1" }
  ],
  issue: {
    kicker: "acme/gateway-api · feat/auth-lockout · mira · GW-731 · 34 files · +1,284 −312",
    standfirst: "…and audit every mutation on the auth surface. One pull request, told as three articles.",
    editors_note: "Three behavior changes ship here, and each gets its own article: a lockout that ends the era of the free four-thousandth guess, an audit event after every mutation on the auth surface, and an exemption that keeps service accounts out of both new mechanisms. One thing does not ship — **AC-6**, the admin unlock the ticket asked for, is implemented nowhere in this diff; the ledger holds that question, and it is the loudest one. A TTL refactor in `session.store.ts` rides along uninvited. Every hunk is accounted for in the appendix; nothing merges unexamined."
  },
  articles: [
    {
      id: "art1",
      kicker: "Article 1 of 3 · Lockout & the 423 path",
      title: "The fifth failure",
      dek: "Priya types one more wrong password and the gateway, for the first time, remembers.",
      files_covered: 9,
      hunks_covered: 21,
      sections: [
        { kind: "prose", text: "At 02:14 on a Tuesday, a script settled onto `POST /login` and began to guess: eleven accounts, four thousand attempts, every one answered with the same flat 401. The four-thousandth wrong password cost the attacker exactly what the first had. That is the incident, and this article is the mechanism that ends it: five failures inside fifteen minutes now lock the account for fifteen more, and a locked account answers 423 with a Retry-After." },
        { kind: "prose", text: "The decision lives across three files, and the fastest way to read them is along the path a request takes. So meet Priya. It is 14:32; she has burned four wrong passwords inside the window, and she is typing a fifth. The footage below is her request, frame by frame — the essay stops telling here and starts showing." },
        { kind: "reenactment" },
        { kind: "standing_question", ref: "q1" },
        { kind: "prose", text: "That is the whole mechanism: a limiter that steps aside for machines, a controller that asks about the lock before it reads the password, a counting service that stamps a clock at five. The way back out is quieter — line 59. When Priya returns at 14:48 with the right password, `verify` succeeds, `lockout.reset(user.id)` runs, and the failure counter dies at zero; the \"reset on success\" e2e pins **AC-3**. What needs a human is the ordering in beat two: a gate that answers 423 before verification says 423 to a correct password too, and a prober who sees 423 where a stranger sees 401 has just learned the account exists." },
        { kind: "standing_question", ref: "q2" },
        { kind: "prose", text: "Underneath, the migration gives `users` two columns and a partial index built with `CREATE INDEX CONCURRENTLY` — which cannot run inside a transaction. Whether the runner wraps each migration file in one is not answerable from this diff." },
        { kind: "exhibit", ref: "e5" },
        { kind: "standing_question", ref: "q7" },
        { kind: "prose", text: "The rest of this article's beat is bookkeeping: `user.model.ts` maps the two new columns, `auth.config.ts` exposes the five-and-fifteen knobs, `.env.example` documents them, and three tests hold the behavior in place. Each is one glance in the appendix." }
      ],
      exhibits: [
        {
          id: "e5",
          file: "db/migrations/0042_add_lockout_columns.sql",
          caption: "Exhibit — the migration",
          permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/db/migrations/0042_add_lockout_columns.sql#L1-L6",
          trimmed_lines: [
            { n: 1, k: "add", t: "ALTER TABLE users" },
            { n: 2, k: "add", t: "  ADD COLUMN failed_attempts integer NOT NULL DEFAULT 0," },
            { n: 3, k: "add", t: "  ADD COLUMN locked_until timestamptz NULL;" },
            { n: 4, k: "add", t: "" },
            { n: 5, k: "add", t: "CREATE INDEX CONCURRENTLY idx_users_locked_until" },
            { n: 6, k: "add", t: "  ON users (locked_until) WHERE locked_until IS NOT NULL;" }
          ],
          full_lines: [
            { n: 1, k: "add", t: "-- GW-731: account lockout columns (new file)" },
            { n: 2, k: "add", t: "-- transaction semantics: decided by the runner, not this file" },
            { n: 3, k: "add", t: "ALTER TABLE users" },
            { n: 4, k: "add", t: "  ADD COLUMN failed_attempts integer NOT NULL DEFAULT 0," },
            { n: 5, k: "add", t: "  ADD COLUMN locked_until timestamptz NULL;" },
            { n: 6, k: "add", t: "" },
            { n: 7, k: "add", t: "CREATE INDEX CONCURRENTLY idx_users_locked_until" },
            { n: 8, k: "add", t: "  ON users (locked_until) WHERE locked_until IS NOT NULL;" },
            { n: 9, k: "add", t: "" },
            { n: 10, k: "add", t: "-- rollback: 0042_down.sql drops both columns and the index" }
          ],
          marks: [
            { line: 5, span: "CONCURRENTLY", note: "`CREATE INDEX CONCURRENTLY` cannot run inside a transaction. Whether the migration runner wraps each file in one is a property of the runner, not of this diff." }
          ],
          note: "`CREATE INDEX CONCURRENTLY` cannot run inside a transaction. Whether the migration runner wraps each file in one is a property of the runner, not of this diff."
        }
      ],
      reenactment: {
        title: "Trace 1 — Priya's fifth wrong password",
        protagonist: "Priya",
        stake: "her fifth wrong password",
        crumbs: ["rate-limit", "login.controller", "lockout.service"],
        frames: [
          {
            id: "A",
            file_tag: "src/middleware/rate-limit.ts · diff",
            permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/src/middleware/rate-limit.ts",
            trimmed_lines: [
              { n: 18, k: "ctx", id: "A18", t: "export function rateLimit(req: Request, res: Response, next: Next) {" },
              { n: 19, k: "add", id: "A19", t: "  if (req.actor?.type === \"service\") return next();        // AC-5" },
              { n: 20, k: "ctx", id: "A20", t: "  const key = `${req.actor?.id ?? req.ip}:${req.route.path}`;" }
            ],
            full_lines: [
              { n: 0, k: "hunk", t: "@@ -18,3 +18,4 @@" },
              { n: 18, k: "ctx", id: "A18", t: "export function rateLimit(req: Request, res: Response, next: Next) {" },
              { n: 19, k: "add", id: "A19", t: "  if (req.actor?.type === \"service\") return next();        // AC-5" },
              { n: 20, k: "ctx", id: "A20", t: "  const key = `${req.actor?.id ?? req.ip}:${req.route.path}`;" },
              { n: 0, k: "hunk", t: "@@ -54,3 +55,4 @@" },
              { n: 55, k: "ctx", t: "  if (count > limits.perMinute) {" },
              { n: 56, k: "add", t: "    metrics.incr(\"rate_limit.throttled\", { route: req.route.path });" },
              { n: 57, k: "ctx", t: "    res.setHeader(\"Retry-After\", \"60\");" },
              { n: 58, k: "ctx", t: "    return res.status(429).end();" }
            ]
          },
          {
            id: "B",
            file_tag: "src/auth/login.controller.ts · diff",
            permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/src/auth/login.controller.ts",
            trimmed_lines: [
              { n: 47, k: "ctx", id: "B47", t: "  async login(req: Request) {" },
              { n: 48, k: "ctx", id: "B48", t: "    const user = await this.users.byEmail(req.body.email);" },
              { n: 49, k: "add", id: "B49", t: "    if (user && (await this.lockout.isLocked(user.id))) {" },
              { n: 50, k: "add", id: "B50", t: "      throw new Locked(retryAfter(user.lockedUntil));        // 423" },
              { n: 51, k: "add", id: "B51", t: "    }" },
              { n: 52, k: "ctx", id: "B52", t: "    if (!user || !(await verify(user, req.body.password))) {" },
              { n: 53, k: "add", id: "B53", t: "      const state = user" },
              { n: 54, k: "add", id: "B54", t: "        ? await this.lockout.recordFailure(user.id, req.ip)" },
              { n: 55, k: "add", id: "B55", t: "        : { locked: false };" },
              { n: 56, k: "add", id: "B56", t: "      if (state.locked) throw new Locked(retryAfter(state.until));" },
              { n: 57, k: "ctx", id: "B57", t: "      throw new Unauthorized();" },
              { n: 58, k: "ctx", id: "B58", t: "    }" },
              { n: 59, k: "add", id: "B59", t: "    await this.lockout.reset(user.id);                       // AC-3" },
              { n: 60, k: "ctx", id: "B60", t: "    return this.sessions.issue(user);" },
              { n: 61, k: "ctx", id: "B61", t: "  }" }
            ],
            full_lines: [
              { n: 0, k: "hunk", t: "@@ -1,4 +1,6 @@" },
              { n: 1, k: "ctx", t: "import { Locked, Unauthorized } from \"../http/errors\";" },
              { n: 2, k: "add", t: "import { retryAfter } from \"../http/retry-after\";" },
              { n: 3, k: "add", t: "import { LockoutService } from \"./lockout.service\";" },
              { n: 0, k: "hunk", t: "@@ -44,8 +47,15 @@" },
              { n: 47, k: "ctx", id: "B47", t: "  async login(req: Request) {" },
              { n: 48, k: "ctx", id: "B48", t: "    const user = await this.users.byEmail(req.body.email);" },
              { n: 49, k: "add", id: "B49", t: "    if (user && (await this.lockout.isLocked(user.id))) {" },
              { n: 50, k: "add", id: "B50", t: "      throw new Locked(retryAfter(user.lockedUntil));        // 423" },
              { n: 51, k: "add", id: "B51", t: "    }" },
              { n: 52, k: "ctx", id: "B52", t: "    if (!user || !(await verify(user, req.body.password))) {" },
              { n: 53, k: "add", id: "B53", t: "      const state = user" },
              { n: 54, k: "add", id: "B54", t: "        ? await this.lockout.recordFailure(user.id, req.ip)" },
              { n: 55, k: "add", id: "B55", t: "        : { locked: false };" },
              { n: 56, k: "add", id: "B56", t: "      if (state.locked) throw new Locked(retryAfter(state.until));" },
              { n: 57, k: "ctx", id: "B57", t: "      throw new Unauthorized();" },
              { n: 58, k: "ctx", id: "B58", t: "    }" },
              { n: 59, k: "add", id: "B59", t: "    await this.lockout.reset(user.id);                       // AC-3" },
              { n: 60, k: "ctx", id: "B60", t: "    return this.sessions.issue(user);" },
              { n: 61, k: "ctx", id: "B61", t: "  }" }
            ]
          },
          {
            id: "C",
            file_tag: "src/auth/lockout.service.ts · new file",
            permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/src/auth/lockout.service.ts",
            trimmed_lines: [
              { n: 1, k: "new", id: "C1", t: "const MAX_ATTEMPTS = 5;              // GW-731: \"five attempts allowed\"" },
              { n: 2, k: "new", id: "C2", t: "const WINDOW_MIN = 15;" },
              { n: 3, k: "new", id: "C3", t: "" },
              { n: 4, k: "new", id: "C4", t: "export class LockoutService {" },
              { n: 5, k: "new", id: "C5", t: "  async recordFailure(userId: string, ip: string): Promise<LockState> {" },
              { n: 6, k: "new", id: "C6", t: "    const failures = await this.store.incrFailures(userId, WINDOW_MIN);" },
              { n: 7, k: "new", id: "C7", t: "    if (failures >= MAX_ATTEMPTS) {" },
              { n: 8, k: "new", id: "C8", t: "      const until = addMinutes(now(), WINDOW_MIN);" },
              { n: 9, k: "new", id: "C9", t: "      await this.store.lock(userId, until);" },
              { n: 10, k: "new", id: "C10", t: "      await this.audit.emitEvent(\"auth.lockout\", { userId, ip, until });" },
              { n: 11, k: "new", id: "C11", t: "      return { locked: true, until };" },
              { n: 12, k: "new", id: "C12", t: "    }" },
              { n: 13, k: "new", id: "C13", t: "    return { locked: false, remaining: MAX_ATTEMPTS - failures };" },
              { n: 14, k: "new", id: "C14", t: "  }" },
              { n: 0, k: "elide", t: "· · ·  isLocked and reset elided — full diff shows them" }
            ],
            full_lines: [
              { n: 1, k: "new", id: "C1", t: "const MAX_ATTEMPTS = 5;              // GW-731: \"five attempts allowed\"" },
              { n: 2, k: "new", id: "C2", t: "const WINDOW_MIN = 15;" },
              { n: 3, k: "new", id: "C3", t: "" },
              { n: 4, k: "new", id: "C4", t: "export class LockoutService {" },
              { n: 5, k: "new", id: "C5", t: "  async recordFailure(userId: string, ip: string): Promise<LockState> {" },
              { n: 6, k: "new", id: "C6", t: "    const failures = await this.store.incrFailures(userId, WINDOW_MIN);" },
              { n: 7, k: "new", id: "C7", t: "    if (failures >= MAX_ATTEMPTS) {" },
              { n: 8, k: "new", id: "C8", t: "      const until = addMinutes(now(), WINDOW_MIN);" },
              { n: 9, k: "new", id: "C9", t: "      await this.store.lock(userId, until);" },
              { n: 10, k: "new", id: "C10", t: "      await this.audit.emitEvent(\"auth.lockout\", { userId, ip, until });" },
              { n: 11, k: "new", id: "C11", t: "      return { locked: true, until };" },
              { n: 12, k: "new", id: "C12", t: "    }" },
              { n: 13, k: "new", id: "C13", t: "    return { locked: false, remaining: MAX_ATTEMPTS - failures };" },
              { n: 14, k: "new", id: "C14", t: "  }" },
              { n: 15, k: "new", t: "" },
              { n: 16, k: "new", t: "  async isLocked(userId: string): Promise<boolean> {" },
              { n: 17, k: "new", t: "    if (await this.accounts.isService(userId)) return false;   // AC-5" },
              { n: 18, k: "new", t: "    const until = await this.store.lockedUntil(userId);" },
              { n: 19, k: "new", t: "    return until !== null && until > now();" },
              { n: 20, k: "new", t: "  }" },
              { n: 21, k: "new", t: "" },
              { n: 22, k: "new", t: "  async reset(userId: string): Promise<void> {" },
              { n: 23, k: "new", t: "    await this.store.clearFailures(userId);                  // AC-3" },
              { n: 24, k: "new", t: "  }" },
              { n: 25, k: "new", t: "}" }
            ]
          }
        ],
        beats: [
          { narration: "`POST /login`, 14:32:07. The limiter looks for a service actor first — Priya is not one — but at three requests a minute she is under its ceiling, therefore it waves her through.", frame: "A", exec: ["A19", "A20"], state: "failures: 4 · locked_until: — · response: —", hot: false },
          { narration: "The controller asks about the lock before it reads the password. `locked_until` is empty, therefore the gate says no — but hold the order in mind: once a lock exists, this line answers 423 even to a correct password.", frame: "B", exec: ["B49"], state: "failures: 4 · locked_until: — · response: —", hot: false },
          { narration: "`verify` chews the fifth wrong password and fails, therefore control drops into the failure branch — which, as of this diff, is no longer a bare 401.", frame: "B", exec: ["B52"], state: "failures: 4 · locked_until: — · response: —", hot: false },
          { narration: "The controller hands the miss to `recordFailure` — cut to `lockout.service`. `incrFailures` bumps the fifteen-minute counter, and four becomes five.", frame: "C", exec: ["C6"], state: "failures: {{4 -> 5}} · locked_until: — · response: —", hot: false },
          { narration: "Five meets five: `failures >= MAX_ATTEMPTS` is true, therefore the account locks and the clock is stamped fifteen minutes out — 14:47.", frame: "C", exec: ["C7", "C8", "C9"], state: "failures: 5 · locked_until: {{— -> 14:47}} · response: —", hot: false },
          { narration: "The first audit event this account has ever produced — `auth.lockout`, with `userId`, `ip`, `until` — goes onto the bus; the lock state returns, the controller throws `Locked`, and the wire answers 423, Retry-After 900.", frame: "C", exec: ["C10", "C11"], state: "failures: 5 · locked_until: 14:47 · response: {{— -> 423 Retry-After 900}}", hot: false },
          { narration: "Hold this frame. `>=` locks on the fifth attempt — all five allowed tries consumed; `>` would have granted a sixth. The ticket says five, the e2e agrees, and still this is the line every reviewer stops on.", frame: "C", exec: ["C7"], state: "failures: 5 · locked_until: 14:47 · response: 423 Retry-After 900", hot: true }
        ]
      },
      files_covered_list: [
        "src/auth/lockout.service.ts", "src/auth/login.controller.ts", "db/migrations/0042_add_lockout_columns.sql",
        "src/models/user.model.ts", "config/auth.config.ts", ".env.example",
        "test/lockout.service.spec.ts", "test/login.flow.e2e.ts", "test/migration.0042.spec.ts"
      ],
      standing_question_refs: ["q1", "q2", "q7"]
    },
    {
      id: "art2",
      kicker: "Article 2 of 3 · Audit events",
      title: "Nineteen echoes",
      dek: "One two-line audit call, said sixteen times verbatim — and the two places it comes out wrong.",
      files_covered: 20,
      hunks_covered: 70,
      sections: [
        { kind: "prose", text: "Every mutation on the auth surface now ends the same way: an event onto the audit bus. The spine is one small new file — `audit.service.ts` publishes action, actor, IP, timestamp, and a redacted detail blob. Hold on the IP: `meta.ip ?? null` is the whole of **AC-4**'s implementation, and no test in this PR asserts the field is ever set. It can regress to null with CI green; half-proven is the ledger's word for it." },
        { kind: "exhibit", ref: "e3" },
        { kind: "prose", text: "From the spine outward, most of this PR's hunks are one sentence said again and again. After every write, the same two lines follow — emit an event, name the actor — verified byte-identical across sixteen controllers, from `mfa` to `api-clients`. Read it once here; the appendix lets you check off the other fifteen without pretending to reread them." },
        { kind: "exhibit", ref: "e4" },
        { kind: "prose", text: "`password.controller.ts` breaks the echo by one line. There the event is emitted before `this.repo.commit()` — so if the commit throws, the ledger remembers a password change that never happened. An audit trail that can record fiction is arguably worse than the silence it replaces." },
        { kind: "exhibit", ref: "ep" },
        { kind: "standing_question", ref: "q3" },
        { kind: "prose", text: "`sso.controller.ts` breaks it by one word. Every other site passes `req.actor.id`; this one passes `req.actor.orgId`, therefore every SSO mutation is attributed to the organization rather than the person who made it — on precisely the surface where, months from now, someone will ask which person." },
        { kind: "exhibit", ref: "es" },
        { kind: "standing_question", ref: "q4" },
        { kind: "prose", text: "Nineteen emitters in all: sixteen echoes, two deviations, and the `auth.lockout` event you watched Article 1 fire." }
      ],
      exhibits: [
        {
          id: "e3",
          file: "src/audit/audit.service.ts",
          caption: "Exhibit — the audit spine (new file)",
          permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/src/audit/audit.service.ts",
          trimmed_lines: [
            { n: 1, k: "new", t: "export class AuditService {" },
            { n: 2, k: "new", t: "  async emitEvent(action: string, meta: AuditMeta): Promise<void> {" },
            { n: 3, k: "new", t: "    await this.bus.publish(\"audit\", {" },
            { n: 4, k: "new", t: "      action," },
            { n: 5, k: "new", t: "      actorId: meta.userId ?? null," },
            { n: 6, k: "new", t: "      ip: meta.ip ?? null,            // AC-4 — no test asserts this is set" },
            { n: 7, k: "new", t: "      at: now().toISOString()," },
            { n: 8, k: "new", t: "      detail: redact(meta)," },
            { n: 9, k: "new", t: "    });" },
            { n: 10, k: "new", t: "  }" },
            { n: 11, k: "new", t: "}" }
          ],
          full_lines: [
            { n: 1, k: "new", t: "import { bus } from \"../infra/bus\";" },
            { n: 2, k: "new", t: "import { redact } from \"./redact\";" },
            { n: 3, k: "new", t: "" },
            { n: 4, k: "new", t: "export class AuditService {" },
            { n: 5, k: "new", t: "  async emitEvent(action: string, meta: AuditMeta): Promise<void> {" },
            { n: 6, k: "new", t: "    await this.bus.publish(\"audit\", {" },
            { n: 7, k: "new", t: "      action," },
            { n: 8, k: "new", t: "      actorId: meta.userId ?? null," },
            { n: 9, k: "new", t: "      ip: meta.ip ?? null,            // AC-4 — no test asserts this is set" },
            { n: 10, k: "new", t: "      at: now().toISOString()," },
            { n: 11, k: "new", t: "      detail: redact(meta)," },
            { n: 12, k: "new", t: "    });" },
            { n: 13, k: "new", t: "  }" },
            { n: 14, k: "new", t: "}" },
            { n: 15, k: "new", t: "" },
            { n: 16, k: "new", t: "function redactKeys(meta: AuditMeta): string[] {" },
            { n: 17, k: "new", t: "  return Object.keys(meta).filter((k) => SENSITIVE.has(k));" },
            { n: 18, k: "new", t: "}" }
          ],
          marks: [
            { line: 6, span: "meta.ip ?? null", note: "This field is AC-4's entire implementation. Written, never asserted — it can regress to null with CI green." }
          ],
          note: "The gold field is AC-4's entire implementation. Written, never asserted."
        },
        {
          id: "e4",
          file: "src/controllers/*.controller.ts",
          caption: "Exhibit — the pattern, one of sixteen identical sites",
          permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/src/controllers/mfa.controller.ts",
          trimmed_lines: [
            { n: 61, k: "ctx", t: "    await this.repo.update(id, changes);" },
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"<resource>.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" },
            { n: 65, k: "ctx", t: "    return ok();" }
          ],
          full_lines: [
            { n: 0, k: "hunk", t: "@@ -58,5 +58,8 @@" },
            { n: 58, k: "ctx", t: "  async update(req: Request) {" },
            { n: 59, k: "ctx", t: "    const { id, changes } = parse(req);" },
            { n: 60, k: "ctx", t: "    await this.authz.assertCanWrite(req.actor, id);" },
            { n: 61, k: "ctx", t: "    await this.repo.update(id, changes);" },
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"<resource>.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" },
            { n: 65, k: "ctx", t: "    return ok();" },
            { n: 66, k: "ctx", t: "  }" }
          ],
          marks: [],
          note: "Byte-identical in mfa, users, orgs, keys, sessions, invites, teams, tokens, webhooks, devices, roles, scopes, billing, profile, admin, api-clients. The two exceptions follow."
        },
        {
          id: "ep",
          file: "src/controllers/password.controller.ts",
          caption: "Exhibit — the emit one line early",
          permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/src/controllers/password.controller.ts",
          trimmed_lines: [
            { n: 132, k: "add", t: "    await this.audit.emitEvent(\"password.update\", {" },
            { n: 133, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 134, k: "add", t: "    });" },
            { n: 135, k: "ctx", t: "    await this.repo.commit();" },
            { n: 136, k: "ctx", t: "    return ok();" }
          ],
          full_lines: [
            { n: 0, k: "hunk", t: "@@ -127,6 +127,9 @@" },
            { n: 127, k: "ctx", t: "  async changePassword(req: Request) {" },
            { n: 128, k: "ctx", t: "    const user = await this.users.byId(req.actor.id);" },
            { n: 129, k: "ctx", t: "    await assertCurrentPassword(user, req.body.current);" },
            { n: 130, k: "ctx", t: "    this.repo.stage(user.id, { passwordHash: await hash(req.body.next) });" },
            { n: 131, k: "ctx", t: "" },
            { n: 132, k: "add", t: "    await this.audit.emitEvent(\"password.update\", {" },
            { n: 133, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 134, k: "add", t: "    });" },
            { n: 135, k: "ctx", t: "    await this.repo.commit();" },
            { n: 136, k: "ctx", t: "    return ok();" },
            { n: 137, k: "ctx", t: "  }" }
          ],
          marks: [
            { line: 132, span: "emitEvent", note: "Every other site emits after its write completes. Here the emit precedes the commit." },
            { line: 135, span: "commit()", note: "A throw between the emit above and this commit records a mutation that never landed." }
          ],
          note: "Every other site emits after its write completes. Here the emit precedes the commit, and a throw between the two records a mutation that never landed."
        },
        {
          id: "es",
          file: "src/controllers/sso.controller.ts",
          caption: "Exhibit — the wrong actor",
          permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/src/controllers/sso.controller.ts",
          trimmed_lines: [
            { n: 94, k: "ctx", t: "    await this.repo.update(id, changes);" },
            { n: 95, k: "add", t: "    await this.audit.emitEvent(\"sso.update\", {" },
            { n: 96, k: "add", t: "      userId: req.actor.orgId, ip: req.ip," },
            { n: 97, k: "add", t: "    });" },
            { n: 98, k: "ctx", t: "    return ok();" }
          ],
          full_lines: [
            { n: 0, k: "hunk", t: "@@ -90,5 +90,8 @@" },
            { n: 90, k: "ctx", t: "  async updateConnection(req: Request) {" },
            { n: 91, k: "ctx", t: "    const { id, changes } = parse(req);" },
            { n: 92, k: "ctx", t: "    await this.authz.assertOrgAdmin(req.actor);" },
            { n: 93, k: "ctx", t: "" },
            { n: 94, k: "ctx", t: "    await this.repo.update(id, changes);" },
            { n: 95, k: "add", t: "    await this.audit.emitEvent(\"sso.update\", {" },
            { n: 96, k: "add", t: "      userId: req.actor.orgId, ip: req.ip," },
            { n: 97, k: "add", t: "    });" },
            { n: 98, k: "ctx", t: "    return ok();" },
            { n: 99, k: "ctx", t: "  }" }
          ],
          marks: [
            { line: 96, span: "orgId", note: "Sixteen sites pass `req.actor.id`. This one passes `orgId`, so the event's actor is the org, not the human." }
          ],
          note: "Sixteen sites pass `req.actor.id`. This one passes `orgId`, so the event's actor is the org, not the human."
        }
      ],
      reenactment: null,
      files_covered_list: [
        "src/audit/audit.service.ts", "src/controllers/password.controller.ts", "src/controllers/sso.controller.ts",
        "src/controllers/mfa.controller.ts", "src/controllers/users.controller.ts", "src/controllers/orgs.controller.ts",
        "src/controllers/keys.controller.ts", "src/controllers/sessions.controller.ts", "src/controllers/invites.controller.ts",
        "src/controllers/teams.controller.ts", "src/controllers/tokens.controller.ts", "src/controllers/webhooks.controller.ts",
        "src/controllers/devices.controller.ts", "src/controllers/roles.controller.ts", "src/controllers/scopes.controller.ts",
        "src/controllers/billing.controller.ts", "src/controllers/profile.controller.ts", "src/controllers/admin.controller.ts",
        "src/controllers/api-clients.controller.ts", "test/audit.service.spec.ts"
      ],
      standing_question_refs: ["q3", "q4"]
    },
    {
      id: "art3",
      kicker: "Article 3 of 3 · Exemptions",
      title: "The ones that must never lock",
      dek: "Service accounts are excused from everything new; four routes never got the memo.",
      files_covered: 2,
      hunks_covered: 8,
      sections: [
        { kind: "prose", text: "The lockout that protects Priya would strangle the deploy bot: a machine retrying a stale token five times a minute would lock itself out of production forever. So service accounts are excused twice. At the rate limiter, line 19 returns `next()` before a key is even built. At the lockout service, `isLocked` answers false for service actors before it reads the store — wrong token or not, a machine sees 401 forever, never 423." },
        { kind: "exhibit", ref: "e6" },
        { kind: "prose", text: "But the exemption reads `req.actor`, and `req.actor` is written by the auth middleware. Four routes mount `rateLimit` before auth; on those routes the actor is undefined at line 19, the exemption never fires, and service traffic falls back to the per-IP bucket — limited like everyone else. The promise \"service accounts are never throttled\" is therefore true on most of the surface, not all of it." },
        { kind: "standing_question", ref: "q6" },
        { kind: "prose", text: "Two files carry all of this — the middleware and its spec. Eight hunks, each one accounted in the appendix; the spec proves the bypass but not the four-route gap." }
      ],
      exhibits: [
        {
          id: "e6",
          file: "src/middleware/rate-limit.ts",
          caption: "Exhibit — the exemption, first layer",
          permalink: "https://github.com/acme/gateway-api/blob/4f2c9e1d7b3a6058e2c1f4a9d8b7c6e5f4a3b2c1/src/middleware/rate-limit.ts",
          trimmed_lines: [
            { n: 18, k: "ctx", t: "export function rateLimit(req: Request, res: Response, next: Next) {" },
            { n: 19, k: "add", t: "  if (req.actor?.type === \"service\") return next();        // AC-5" },
            { n: 20, k: "ctx", t: "  const key = `${req.actor?.id ?? req.ip}:${req.route.path}`;" }
          ],
          full_lines: [
            { n: 0, k: "hunk", t: "@@ -18,3 +18,4 @@" },
            { n: 18, k: "ctx", t: "export function rateLimit(req: Request, res: Response, next: Next) {" },
            { n: 19, k: "add", t: "  if (req.actor?.type === \"service\") return next();        // AC-5" },
            { n: 20, k: "ctx", t: "  const key = `${req.actor?.id ?? req.ip}:${req.route.path}`;" },
            { n: 21, k: "ctx", t: "  const count = bucket.incr(key, 60);" },
            { n: 0, k: "hunk", t: "@@ -54,3 +55,4 @@" },
            { n: 55, k: "ctx", t: "  if (count > limits.perMinute) {" },
            { n: 56, k: "add", t: "    metrics.incr(\"rate_limit.throttled\", { route: req.route.path });" },
            { n: 57, k: "ctx", t: "    res.setHeader(\"Retry-After\", \"60\");" },
            { n: 58, k: "ctx", t: "    return res.status(429).end();" }
          ],
          marks: [
            { line: 19, span: "req.actor?.type", note: "The exemption reads `req.actor`, written by the auth middleware. On routes that mount rateLimit before auth, this is undefined and the exemption never fires." }
          ],
          note: "The second layer is `isLocked`'s first line, shown in Article 1's full diff: service actors short-circuit to false."
        }
      ],
      reenactment: null,
      files_covered_list: ["src/middleware/rate-limit.ts", "test/rate-limit.spec.ts"],
      standing_question_refs: ["q6"]
    }
  ],
  standing_questions: [
    {
      id: "q1", text: "`>=` locks on the fifth attempt, all five allowed tries consumed; `>` would grant a sixth. Is this the fence-post the ticket meant?",
      raised_in: "art1", raised_note: "at the freeze-frame",
      draft: "Boundary: failures >= MAX_ATTEMPTS locks on the fifth attempt (all five consumed); '>' would allow a sixth. Confirm this is the fence-post GW-731 meant.",
      hangs_on: { files: ["src/auth/lockout.service.ts"], tokens: [">=", "MAX_ATTEMPTS"] },
      backing_hunks: ["src/auth/lockout.service.ts#1"],
      author_answer: null
    },
    {
      id: "q2", text: "The lock check answers 423 before the password is verified — a 423 confirms the account exists where a stranger sees 401. Accept the enumeration tradeoff?",
      raised_in: "art1", raised_note: "",
      draft: "Ordering: the lock check answers 423 before password verification, so a 423 confirms the account exists (vs 401 for strangers). Accept the enumeration tradeoff?",
      hangs_on: { files: ["src/auth/login.controller.ts"], tokens: ["isLocked", "verify"] },
      backing_hunks: ["src/auth/login.controller.ts#2"],
      author_answer: null
    },
    {
      id: "q3", text: "`password.controller.ts` emits its audit event before `repo.commit()`; a failed commit records a change that never happened. Move the emit?",
      raised_in: "art2", raised_note: "",
      draft: "password.controller.ts: audit event emitted before repo.commit() — a failed commit records a change that never happened. Move the emit after the commit?",
      hangs_on: { files: ["src/controllers/password.controller.ts"], tokens: ["emitEvent", "commit()"] },
      backing_hunks: ["src/controllers/password.controller.ts#1"],
      author_answer: null
    },
    {
      id: "q4", text: "`sso.controller.ts` passes `req.actor.orgId` where sixteen sites pass `req.actor.id` — the actor recorded is the org, not the person. Typo or intent?",
      raised_in: "art2", raised_note: "",
      draft: "sso.controller.ts: req.actor.orgId passed where sixteen sites pass req.actor.id — SSO mutations attributed to the org, not the person. Typo or intent?",
      hangs_on: { files: ["src/controllers/sso.controller.ts"], tokens: ["orgId"] },
      backing_hunks: ["src/controllers/sso.controller.ts#1"],
      author_answer: null
    },
    {
      id: "q5", text: "AC-6 asked for an admin unlock; nothing in the diff implements it, so every lock waits out its fifteen minutes. Ship without it, or block?",
      raised_in: "cover", raised_note: "in the editor's note",
      draft: "AC-6: no admin unlock endpoint anywhere in the diff; every lock waits out its 15 minutes. Ship without it, or block?",
      hangs_on: { files: [], tokens: ["AC-6"] },
      backing_hunks: [],
      author_answer: null
    },
    {
      id: "q6", text: "Four routes mount `rateLimit` ahead of auth — `req.actor` is undefined there and the service exemption never fires. Reorder the middleware, or accept the gap?",
      raised_in: "art3", raised_note: "",
      draft: "rate-limit.ts: four routes mount the limiter before auth, so req.actor is undefined and the service exemption never fires there. Reorder or accept?",
      hangs_on: { files: ["src/middleware/rate-limit.ts"], tokens: ["req.actor"] },
      backing_hunks: ["src/middleware/rate-limit.ts#1"],
      author_answer: null
    },
    {
      id: "q7", text: "`CREATE INDEX CONCURRENTLY` cannot run inside a transaction — does the migration runner wrap each file in one? Not answerable from the diff.",
      raised_in: "art1", raised_note: "",
      draft: "migration 0042: CREATE INDEX CONCURRENTLY cannot run inside a transaction — confirm the migration runner does not wrap each file.",
      hangs_on: { files: ["db/migrations/0042_add_lockout_columns.sql"], tokens: ["CONCURRENTLY"] },
      backing_hunks: ["db/migrations/0042_add_lockout_columns.sql#1"],
      author_answer: null
    }
  ],
  judgment_calls: [
    { text: "AC-6 is not implemented — ship without it or block the PR?", source: "Standing question 5", why: "No code in these thirty-four files implements an unlock; every lock is undone only by fifteen minutes of waiting, or by a DBA with an UPDATE statement." },
    { text: "The `session.store.ts` refactor is separable — ask the author to split it out?", source: "appendix, mechanical remainder", why: "TTL logic extracted into `ttlFor()`; the author claims no behavior change and it shares no symbol with the feature." },
    { text: "`CREATE INDEX CONCURRENTLY` inside the runner's transaction — verify before merge?", source: "Standing question 7", why: "A property of the runner, not the diff. One message to whoever owns it settles it." },
    { text: "423-before-verify confirms account existence — accept the enumeration tradeoff?", source: "Standing question 2", why: "Deliberate or not, a 423 where a stranger sees 401 is an enumeration oracle." }
  ],
  appendix: {
    total_rows: 38,
    groups: [
      {
        id: "shown", title: "Shown in the articles", files: 7, hunks: 27,
        why: "You have already read these — each row reopens what its article displayed, plus the surrounding diff.",
        template: null, mark_all_label: null,
        rows: [
          { path: "src/auth/lockout.service.ts", hunks: 6, hash: "sha256:1a2b3c4d", what_changed: "new file — the counting service", article_ref: "art1", article_label: "Article 1", diff_lines: [
            { n: 1, k: "new", t: "const MAX_ATTEMPTS = 5;              // GW-731: \"five attempts allowed\"" },
            { n: 2, k: "new", t: "const WINDOW_MIN = 15;" },
            { n: 6, k: "new", t: "    const failures = await this.store.incrFailures(userId, WINDOW_MIN);" },
            { n: 7, k: "new", t: "    if (failures >= MAX_ATTEMPTS) {" },
            { n: 9, k: "new", t: "      await this.store.lock(userId, until);" },
            { n: 10, k: "new", t: "      await this.audit.emitEvent(\"auth.lockout\", { userId, ip, until });" },
            { n: 0, k: "elide", t: "· · ·  full file shown in Article 1 (full diff)" }
          ] },
          { path: "src/auth/login.controller.ts", hunks: 4, hash: "sha256:2b3c4d5e", what_changed: "the gate: 423 before verify, reset on success", article_ref: "art1", article_label: "Article 1", diff_lines: [
            { n: 49, k: "add", t: "    if (user && (await this.lockout.isLocked(user.id))) {" },
            { n: 50, k: "add", t: "      throw new Locked(retryAfter(user.lockedUntil));        // 423" },
            { n: 56, k: "add", t: "      if (state.locked) throw new Locked(retryAfter(state.until));" },
            { n: 59, k: "add", t: "    await this.lockout.reset(user.id);                       // AC-3" },
            { n: 0, k: "elide", t: "· · ·  walked beat-by-beat in Article 1" }
          ] },
          { path: "db/migrations/0042_add_lockout_columns.sql", hunks: 1, hash: "sha256:3c4d5e6f", what_changed: "two columns + partial index", article_ref: "art1", article_label: "Article 1", diff_lines: [
            { n: 1, k: "add", t: "ALTER TABLE users" },
            { n: 2, k: "add", t: "  ADD COLUMN failed_attempts integer NOT NULL DEFAULT 0," },
            { n: 3, k: "add", t: "  ADD COLUMN locked_until timestamptz NULL;" },
            { n: 5, k: "add", t: "CREATE INDEX CONCURRENTLY idx_users_locked_until" },
            { n: 6, k: "add", t: "  ON users (locked_until) WHERE locked_until IS NOT NULL;" }
          ] },
          { path: "src/audit/audit.service.ts", hunks: 3, hash: "sha256:4d5e6f70", what_changed: "new file — the audit spine", article_ref: "art2", article_label: "Article 2", diff_lines: [
            { n: 2, k: "new", t: "  async emitEvent(action: string, meta: AuditMeta): Promise<void> {" },
            { n: 5, k: "new", t: "      actorId: meta.userId ?? null," },
            { n: 6, k: "new", t: "      ip: meta.ip ?? null,            // AC-4 — no test asserts this is set" },
            { n: 8, k: "new", t: "      detail: redact(meta)," },
            { n: 0, k: "elide", t: "· · ·  exhibited in Article 2" }
          ] },
          { path: "src/controllers/password.controller.ts", hunks: 4, hash: "sha256:5e6f7081", what_changed: "deviation: emit before commit", article_ref: "art2", article_label: "Article 2", diff_lines: [
            { n: 132, k: "add", t: "    await this.audit.emitEvent(\"password.update\", {" },
            { n: 133, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 134, k: "add", t: "    });" },
            { n: 135, k: "ctx", t: "    await this.repo.commit();" }
          ] },
          { path: "src/controllers/sso.controller.ts", hunks: 3, hash: "sha256:6f708192", what_changed: "deviation: orgId recorded as the actor", article_ref: "art2", article_label: "Article 2", diff_lines: [
            { n: 95, k: "add", t: "    await this.audit.emitEvent(\"sso.update\", {" },
            { n: 96, k: "add", t: "      userId: req.actor.orgId, ip: req.ip," },
            { n: 97, k: "add", t: "    });" }
          ] },
          { path: "src/middleware/rate-limit.ts", hunks: 6, hash: "sha256:708192a3", what_changed: "service bypass + per-IP fallback", article_ref: "art3", article_label: "Article 3", diff_lines: [
            { n: 18, k: "ctx", t: "export function rateLimit(req: Request, res: Response, next: Next) {" },
            { n: 19, k: "add", t: "  if (req.actor?.type === \"service\") return next();        // AC-5" },
            { n: 20, k: "ctx", t: "  const key = `${req.actor?.id ?? req.ip}:${req.route.path}`;" }
          ] }
        ]
      },
      {
        id: "pattern", title: "Pattern-verified", files: 16, hunks: 58,
        why: "One template, sixteen instances, hash-verified identical. Read the template once; check off the instances without pretending to reread them.",
        template: { caption: "The template — identical at every site", lines: [
          { n: 61, k: "ctx", t: "    await this.repo.update(id, changes);" },
          { n: 62, k: "add", t: "    await this.audit.emitEvent(\"<resource>.update\", {" },
          { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
          { n: 64, k: "add", t: "    });" },
          { n: 65, k: "ctx", t: "    return ok();" }
        ] },
        mark_all_label: "The template is the review — mark all sixteen seen",
        rows: [
          { path: "src/controllers/mfa.controller.ts", hunks: 3, hash: "sha256:a1", what_changed: "identical — hash-verified", diff_lines: [
            { n: 61, k: "ctx", t: "    await this.repo.update(id, changes);" },
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"mfa.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" },
            { n: 65, k: "ctx", t: "    return ok();" }
          ] },
          { path: "src/controllers/users.controller.ts", hunks: 6, hash: "sha256:a2", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"users.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/orgs.controller.ts", hunks: 5, hash: "sha256:a3", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"orgs.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/keys.controller.ts", hunks: 4, hash: "sha256:a4", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"keys.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/sessions.controller.ts", hunks: 4, hash: "sha256:a5", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"sessions.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/invites.controller.ts", hunks: 4, hash: "sha256:a6", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"invites.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/teams.controller.ts", hunks: 4, hash: "sha256:a7", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"teams.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/tokens.controller.ts", hunks: 4, hash: "sha256:a8", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"tokens.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/webhooks.controller.ts", hunks: 4, hash: "sha256:a9", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"webhooks.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/devices.controller.ts", hunks: 3, hash: "sha256:aa", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"devices.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/roles.controller.ts", hunks: 3, hash: "sha256:ab", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"roles.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/scopes.controller.ts", hunks: 3, hash: "sha256:ac", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"scopes.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/billing.controller.ts", hunks: 3, hash: "sha256:ad", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"billing.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/profile.controller.ts", hunks: 3, hash: "sha256:ae", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"profile.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/admin.controller.ts", hunks: 3, hash: "sha256:af", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"admin.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] },
          { path: "src/controllers/api-clients.controller.ts", hunks: 2, hash: "sha256:b0", what_changed: "identical — hash-verified", diff_lines: [
            { n: 62, k: "add", t: "    await this.audit.emitEvent(\"api-clients.update\", {" },
            { n: 63, k: "add", t: "      userId: req.actor.id, ip: req.ip," },
            { n: 64, k: "add", t: "    });" }
          ] }
        ]
      },
      {
        id: "mechanical", title: "Mechanical remainder", files: 10, hunks: 17,
        why: "Knobs, mappings, and the tests that hold the behavior in place — plus the one refactor nobody ordered. One line each; the diff is there if the line is not enough.",
        template: null, mark_all_label: null,
        rows: [
          { path: "src/models/user.model.ts", hunks: 2, hash: "sha256:c1", what_changed: "maps failed_attempts and locked_until onto the model", diff_lines: [
            { n: 24, k: "add", t: "  failedAttempts: integer(\"failed_attempts\").notNull().default(0)," },
            { n: 25, k: "add", t: "  lockedUntil: timestamp(\"locked_until\", { withTimezone: true })," }
          ] },
          { path: "config/auth.config.ts", hunks: 2, hash: "sha256:c2", what_changed: "exposes the five-and-fifteen knobs", diff_lines: [
            { n: 31, k: "add", t: "  lockout: {" },
            { n: 32, k: "add", t: "    maxAttempts: env.int(\"AUTH_LOCKOUT_MAX_ATTEMPTS\", 5)," },
            { n: 33, k: "add", t: "    windowMin: env.int(\"AUTH_LOCKOUT_WINDOW_MIN\", 15)," },
            { n: 34, k: "add", t: "  }," }
          ] },
          { path: ".env.example", hunks: 1, hash: "sha256:c3", what_changed: "documents the lockout defaults", diff_lines: [
            { n: 12, k: "add", t: "# Account lockout (GW-731)" },
            { n: 13, k: "add", t: "AUTH_LOCKOUT_MAX_ATTEMPTS=5" },
            { n: 14, k: "add", t: "AUTH_LOCKOUT_WINDOW_MIN=15" }
          ] },
          { path: "src/session/session.store.ts", hunks: 2, hash: "sha256:c4", what_changed: "uninvited: TTL logic extracted into ttlFor() — no ticket asked; separable", diff_lines: [
            { n: 41, k: "del", t: "    const ttl = this.config.sessionTtlMin * 60;" },
            { n: 41, k: "add", t: "    return this.redis.set(key, value, \"EX\", ttlFor(this.config));" },
            { n: 58, k: "add", t: "function ttlFor(config: SessionConfig): number {" },
            { n: 59, k: "add", t: "  return config.sessionTtlMin * 60;" },
            { n: 60, k: "add", t: "}" }
          ] },
          { path: "test/lockout.service.spec.ts", hunks: 2, hash: "sha256:c5", what_changed: "locks on the fifth consecutive failure (AC-1)", diff_lines: [
            { n: 8, k: "add", t: "it(\"locks on the fifth consecutive failure\", async () => {" },
            { n: 9, k: "add", t: "  for (let i = 0; i < 5; i++) await svc.recordFailure(u.id, ip);" },
            { n: 10, k: "add", t: "  expect(await svc.isLocked(u.id)).toBe(true);" },
            { n: 11, k: "add", t: "});" }
          ] },
          { path: "test/login.flow.e2e.ts", hunks: 2, hash: "sha256:c6", what_changed: "locked account gets 423 (AC-2) · reset on success (AC-3)", diff_lines: [
            { n: 21, k: "add", t: "it(\"locked account gets 423\", async () => {" },
            { n: 22, k: "add", t: "  const res = await post(\"/login\", wrongPassword);" },
            { n: 23, k: "add", t: "  expect(res.status).toBe(423);" },
            { n: 24, k: "add", t: "  expect(res.headers[\"retry-after\"]).toBe(\"900\");" },
            { n: 25, k: "add", t: "});" }
          ] },
          { path: "test/audit.service.spec.ts", hunks: 2, hash: "sha256:c7", what_changed: "asserts the event shape — not that ip is ever set (AC-4 gap)", diff_lines: [
            { n: 14, k: "add", t: "it(\"publishes action, actor, timestamp\", async () => {" },
            { n: 15, k: "add", t: "  await svc.emitEvent(\"users.update\", { userId: \"u1\", ip: \"10.0.0.9\" });" },
            { n: 16, k: "add", t: "  expect(bus.last(\"audit\").action).toBe(\"users.update\");" },
            { n: 17, k: "add", t: "});" }
          ] },
          { path: "test/rate-limit.spec.ts", hunks: 2, hash: "sha256:c8", what_changed: "service actors bypass the limiter (AC-5) — the four-route gap is untested", diff_lines: [
            { n: 9, k: "add", t: "it(\"service actors bypass the limiter\", async () => {" },
            { n: 10, k: "add", t: "  req.actor = { id: \"bot\", type: \"service\" };" },
            { n: 11, k: "add", t: "  rateLimit(req, res, next);" },
            { n: 12, k: "add", t: "  expect(next).toHaveBeenCalled();" },
            { n: 13, k: "add", t: "});" }
          ] },
          { path: "test/session.store.spec.ts", hunks: 1, hash: "sha256:c9", what_changed: "ttlFor parity with the old inline computation", diff_lines: [
            { n: 6, k: "add", t: "it(\"ttlFor matches the previous inline computation\", () => {" },
            { n: 7, k: "add", t: "  expect(ttlFor({ sessionTtlMin: 30 })).toBe(1800);" },
            { n: 8, k: "add", t: "});" }
          ] },
          { path: "test/migration.0042.spec.ts", hunks: 1, hash: "sha256:ca", what_changed: "columns and partial index exist after migrate", diff_lines: [
            { n: 5, k: "add", t: "it(\"adds lockout columns and partial index\", async () => {" },
            { n: 6, k: "add", t: "  const cols = await introspect(\"users\");" },
            { n: 7, k: "add", t: "  expect(cols).toContain(\"locked_until\");" },
            { n: 8, k: "add", t: "});" }
          ] }
        ]
      },
      {
        id: "trusted", title: "Trusted", files: 5, hunks: 8,
        why: "Generated or mechanical by construction — verify the generator, not the output.",
        template: null,
        mark_all_label: "Acknowledge the bundle — mark all five seen",
        rows: [
          { path: "pnpm-lock.yaml", hunks: 3, hash: "sha256:d1", what_changed: "+412 −6 · lockfile follows package.json", diff_lines: [
            { n: 1, k: "ctx", t: "lockfileVersion: '9.0'" },
            { n: 0, k: "elide", t: "· · ·  +412 −6 generated lines — verify package.json, not this" }
          ] },
          { path: "openapi.generated.json", hunks: 2, hash: "sha256:d2", what_changed: "regenerated from route annotations", diff_lines: [
            { n: 2, k: "add", t: "  \"x-generated\": \"2026-07-28T14:47:03Z\"," },
            { n: 0, k: "elide", t: "· · ·  regenerated — diff follows the annotations" }
          ] },
          { path: "__snapshots__/auth.snap", hunks: 1, hash: "sha256:d3", what_changed: "regenerated — 423 body shape", diff_lines: [
            { n: 3, k: "add", t: "exports[\"locked account gets 423 1\"] = \"Retry-After: 900\";" }
          ] },
          { path: "__snapshots__/audit.snap", hunks: 1, hash: "sha256:d4", what_changed: "regenerated — audit event shape", diff_lines: [
            { n: 7, k: "add", t: "exports[\"publishes action, actor, timestamp 1\"] = \"users.update\";" }
          ] },
          { path: "CHANGELOG.md", hunks: 1, hash: "sha256:d5", what_changed: "release notes entry", diff_lines: [
            { n: 4, k: "add", t: "## 1.42.0 — lock accounts after repeated failed logins (GW-731)" }
          ] }
        ]
      }
    ]
  },
  trust_bundle: {
    files: [
      { path: "pnpm-lock.yaml", hunks: 3, why: "lockfile" },
      { path: "openapi.generated.json", hunks: 2, why: "regenerated" },
      { path: "__snapshots__/auth.snap", hunks: 1, why: "regenerated" },
      { path: "__snapshots__/audit.snap", hunks: 1, why: "regenerated" },
      { path: "CHANGELOG.md", hunks: 1, why: "docs" }
    ]
  },
  coverage: {
    total_hunks: 110,
    shown_in_articles: 27,
    pattern_verified: 58,
    mechanical_remainder: 17,
    trusted: 8,
    note: "The articles show the 27 that need a human; the appendix makes the other 83 tractable."
  },
  delta: null
};
