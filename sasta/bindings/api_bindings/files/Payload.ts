// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { ListView } from "./ListView";

export type Payload = { "type": "FilePaths", "content": ListView } | { "type": "Error", "content": { code: number, message: string, } };
