import { writable } from 'svelte/store';
import type { DiagnosticEvent } from '$lib/types';

export const diagnostics = writable<DiagnosticEvent[]>([]);
