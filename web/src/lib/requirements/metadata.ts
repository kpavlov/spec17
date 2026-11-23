// Segment keys reserve "all" for clearing; values may contain any text.
export const metadataKey = (value: string) => `value:${value}`;
export const metadataValue = (key: string) => key === "all" ? undefined : key.slice(6);
