import { describe, expect, it } from "vitest";
import { hasFields, isListOf, isNullable, isNumber, isOneOf, isRecord, isString } from "./guards";

describe("values arriving from the app are told apart by shape", () => {
  it("recognises an object and nothing else as a record", () => {
    expect(isRecord({ a: 1 })).toBe(true);
    expect(isRecord(null)).toBe(false);
    expect(isRecord("text")).toBe(false);
    expect(isRecord(7)).toBe(false);
  });

  it("recognises a number only when it is a number", () => {
    expect(isNumber(3)).toBe(true);
    expect(isNumber("3")).toBe(false);
  });

  it("recognises a string only when it is a string", () => {
    expect(isString("3")).toBe(true);
    expect(isString(3)).toBe(false);
  });

  it("accepts only the listed words", () => {
    const isColour = isOneOf(["red", "blue"]);

    expect(isColour("red")).toBe(true);
    expect(isColour("green")).toBe(false);
    expect(isColour(5)).toBe(false);
  });

  it("accepts a nullable value when it is null or passes the guard", () => {
    const isMaybeNumber = isNullable(isNumber);

    expect(isMaybeNumber(null)).toBe(true);
    expect(isMaybeNumber(4)).toBe(true);
    expect(isMaybeNumber("4")).toBe(false);
  });

  it("accepts a list only when every item passes the guard", () => {
    const isNumbers = isListOf(isNumber);

    expect(isNumbers([1, 2])).toBe(true);
    expect(isNumbers([1, "2"])).toBe(false);
    expect(isNumbers("12")).toBe(false);
  });

  it("requires every named field to pass its guard", () => {
    const fields = { name: isString, age: isNumber };

    expect(hasFields({ name: "Ada", age: 36 }, fields)).toBe(true);
    expect(hasFields({ name: "Ada" }, fields)).toBe(false);
    expect(hasFields(null, fields)).toBe(false);
  });
});
