import test from "node:test";
import assert from "node:assert/strict";
import { resettable } from "../src/lib/dbError.ts";

// [WEB-72]
test("a reset is offered only for failures that deleting the database fixes", () => {
  for (const name of ["VersionError", "NotFoundError", "UnknownError"]) assert.equal(resettable(new DOMException("x", name)), true, name);
  for (const name of ["SecurityError", "InvalidStateError", "QuotaExceededError", "AbortError", "DataCloneError"]) assert.equal(resettable(new DOMException("x", name)), false, name);
  for (const e of [new Error("storage blocked by another tab"), null, undefined]) assert.equal(resettable(e), false);
});
