import { writable, derived } from 'svelte/store';

// Types
export interface FileInfo {
    path: string;
    name: string;
    size: number;
    fileType: 'Csv' | 'Excel' | 'Pdf' | 'Text' | 'Unknown';
    isValid: boolean;
    error?: string | null;
}

export interface TemplateFile {
    path: string;
    name: string;
    content: string;
}

export interface AppStatus {
    filesLoaded: number;
    validationStatus: 'NotStarted' | 'InProgress' | 'Valid' | 'Invalid';
    apiConnected: boolean;
    readyToSend: boolean;
}

// Stores
export const files = writable<FileInfo[]>([]);
export const template = writable<TemplateFile | null>(null);
export const apiKey = writable<string>('');
export const appStatus = writable<AppStatus>({
    filesLoaded: 0,
    validationStatus: 'NotStarted',
    apiConnected: false,
    readyToSend: false
});
export const isProcessing = writable<boolean>(false);

// Derived stores
export const fileCount = derived(files, $files => $files.length);
export const hasFiles = derived(files, $files => $files.length > 0);
export const hasTemplate = derived(template, $template => $template !== null);
export const allFilesValid = derived(files, $files => $files.every(f => f.isValid));

// Actions
export const addFile = (file: FileInfo) => {
    files.update(current => [...current, file]);
};

export const removeFile = (path: string) => {
    files.update(current => current.filter(f => f.path !== path));
};

export const clearFiles = () => {
    files.set([]);
};

export const setTemplate = (templateFile: TemplateFile) => {
    template.set(templateFile);
};

export const clearTemplate = () => {
    template.set(null);
};

export const setApiKey = (key: string) => {
    apiKey.set(key);
};

export const clearApiKey = () => {
    apiKey.set('');
};
