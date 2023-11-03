import type { ToastStore } from "@skeletonlabs/skeleton";
import { writable, type Writable } from "svelte/store";

export const toastStore: Writable<ToastStore> = writable();