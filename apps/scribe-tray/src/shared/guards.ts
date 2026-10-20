export type Guard<T> = (value: unknown) => value is T;

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function hasFields(
  value: unknown,
  fields: Readonly<Record<string, Guard<unknown>>>,
): value is Record<string, unknown> {
  return isRecord(value) && Object.entries(fields).every(([name, guard]) => guard(value[name]));
}

export function isOneOf<T extends string>(allowed: readonly T[]): Guard<T> {
  return (value: unknown): value is T =>
    typeof value === "string" && allowed.some((option) => option === value);
}

export function isString(value: unknown): value is string {
  return typeof value === "string";
}

export function isNumber(value: unknown): value is number {
  return typeof value === "number";
}

export function isNullable<T>(guard: Guard<T>): Guard<T | null> {
  return (value: unknown): value is T | null => value === null || guard(value);
}

export function isListOf<T>(guard: Guard<T>): Guard<readonly T[]> {
  return (value: unknown): value is readonly T[] => Array.isArray(value) && value.every(guard);
}

export function expecting<T>(guard: Guard<T>, what: string): (value: unknown) => T {
  return (value) => {
    if (!guard(value)) {
      throw new TypeError(`Unexpected ${what} from Scribe`);
    }
    return value;
  };
}
