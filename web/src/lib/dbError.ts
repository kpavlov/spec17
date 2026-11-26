// (WEB-72) the IndexedDB failures that deleting the database fixes: a newer spec17 made it, a store is missing, or the browser cannot read it.
// Blocked or unavailable storage is none of these: there a reset would delete data or do nothing, and help with neither.
const RESETTABLE = ["VersionError", "NotFoundError", "UnknownError"];
export const resettable = (e: unknown): boolean => RESETTABLE.includes((e as { name?: string } | null)?.name ?? "");
