import browser from "webextension-polyfill";
import { openDB } from "idb";

import type {
  BrowserPackFile,
  BrowserPackManifest,
  BrowserPackStatus,
  FlowUiSettings,
  FlowStorageBackend,
  FlowWorkbenchDraft,
} from "./protocol";

const DB_NAME = "flow-browser-runtime";
const DB_VERSION = 1;
const STORE_FILES = "files";
const STORE_MANIFESTS = "manifests";
const STORE_SETTINGS = "settings";

const EXT_FILE_PREFIX = "flow:file:";
const EXT_MANIFEST_PREFIX = "flow:manifest:";
const EXT_SETTING_PREFIX = "flow:setting:";

const DEFAULT_UI_SETTINGS: FlowUiSettings = {
  localOnly: true,
  defaultTask: "rewrite-selection",
  autoApplyRewrite: true,
  captureActiveTabContext: true,
  preferredChatModel: "qwen3-0.6b",
  preferredOcrModel: "trocr-small-printed",
  preferredVisionLanguageModel: "qwen3.5-0.8b",
};

const DEFAULT_WORKBENCH_DRAFT: FlowWorkbenchDraft = {
  task: "rewrite-selection",
  prompt: "Rewrite the selected text so it is clearer and tighter.",
  imageSources: "",
};

type StoredFileRecord = {
  key: string;
  bytes?: ArrayBuffer;
  base64?: string;
  contentType: string;
  updatedAt: number;
  sizeBytes: number;
  sha256: string | null;
  opfs: boolean;
};

type StoredManifestRecord = {
  key: string;
  manifest: BrowserPackManifest;
  updatedAt: number;
};

type VerifyState = "ready" | "missing" | "corrupt";

function fileKey(packKey: string, filePath: string) {
  return `${packKey}:${filePath}`;
}

function extFileKey(packKey: string, filePath: string) {
  return `${EXT_FILE_PREFIX}${fileKey(packKey, filePath)}`;
}

function extManifestKey(packKey: string) {
  return `${EXT_MANIFEST_PREFIX}${packKey}`;
}

function guessContentType(filePath: string) {
  if (filePath.endsWith(".json")) {
    return "application/json";
  }
  if (filePath.endsWith(".txt")) {
    return "text/plain";
  }
  return "application/octet-stream";
}

function canUseIndexedDb() {
  return typeof indexedDB !== "undefined";
}

function canUseExtensionStorage() {
  return !!browser?.storage?.local;
}

function toBase64(bytes: ArrayBuffer) {
  let binary = "";
  const view = new Uint8Array(bytes);
  for (let index = 0; index < view.length; index += 1) {
    binary += String.fromCharCode(view[index]);
  }
  return btoa(binary);
}

function fromBase64(encoded: string) {
  const binary = atob(encoded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes.buffer;
}

async function sha256Hex(bytes: ArrayBuffer) {
  if (!globalThis.crypto?.subtle) {
    return null;
  }

  const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest))
    .map((value) => value.toString(16).padStart(2, "0"))
    .join("");
}

function normalizeUiSettings(value: unknown): FlowUiSettings {
  const record = typeof value === "object" && value ? (value as Partial<FlowUiSettings>) : {};
  return {
    localOnly: record.localOnly ?? DEFAULT_UI_SETTINGS.localOnly,
    defaultTask: record.defaultTask ?? DEFAULT_UI_SETTINGS.defaultTask,
    autoApplyRewrite: record.autoApplyRewrite ?? DEFAULT_UI_SETTINGS.autoApplyRewrite,
    captureActiveTabContext:
      record.captureActiveTabContext ?? DEFAULT_UI_SETTINGS.captureActiveTabContext,
    preferredChatModel: record.preferredChatModel ?? DEFAULT_UI_SETTINGS.preferredChatModel,
    preferredOcrModel: record.preferredOcrModel ?? DEFAULT_UI_SETTINGS.preferredOcrModel,
    preferredVisionLanguageModel:
      record.preferredVisionLanguageModel ??
      DEFAULT_UI_SETTINGS.preferredVisionLanguageModel,
  };
}

function normalizeWorkbenchDraft(value: unknown): FlowWorkbenchDraft {
  const record = typeof value === "object" && value ? (value as Partial<FlowWorkbenchDraft>) : {};
  return {
    task: record.task ?? DEFAULT_WORKBENCH_DRAFT.task,
    prompt: record.prompt ?? DEFAULT_WORKBENCH_DRAFT.prompt,
    imageSources: record.imageSources ?? DEFAULT_WORKBENCH_DRAFT.imageSources,
  };
}

export class FlowBrowserStorage {
  async backend(): Promise<FlowStorageBackend> {
    if (
      typeof navigator !== "undefined" &&
      navigator.storage &&
      typeof navigator.storage.getDirectory === "function"
    ) {
      return "opfs";
    }

    if (canUseIndexedDb()) {
      return "indexeddb";
    }

    return "extension-storage";
  }

  async setLocalOnly(localOnly: boolean) {
    await this.updateSettings({ localOnly });
  }

  async getLocalOnly(): Promise<boolean> {
    return (await this.getSettings()).localOnly;
  }

  async getSettings(): Promise<FlowUiSettings> {
    const db = await this.maybeDb();
    if (db) {
      const stored = await db.get(STORE_SETTINGS, "uiSettings");
      const settings = normalizeUiSettings(stored);
      const legacyLocalOnly = await db.get(STORE_SETTINGS, "localOnly");
      return legacyLocalOnly == null
        ? settings
        : normalizeUiSettings({ ...settings, localOnly: Boolean(legacyLocalOnly) });
    }

    if (canUseExtensionStorage()) {
      const stored = await browser.storage.local.get([
        `${EXT_SETTING_PREFIX}uiSettings`,
        `${EXT_SETTING_PREFIX}localOnly`,
      ]);
      return normalizeUiSettings({
        ...(stored[`${EXT_SETTING_PREFIX}uiSettings`] as Partial<FlowUiSettings> | undefined),
        localOnly:
          (stored[`${EXT_SETTING_PREFIX}localOnly`] as boolean | undefined) ??
          DEFAULT_UI_SETTINGS.localOnly,
      });
    }

    return { ...DEFAULT_UI_SETTINGS };
  }

  async updateSettings(patch: Partial<FlowUiSettings>): Promise<FlowUiSettings> {
    const current = await this.getSettings();
    const next = normalizeUiSettings({ ...current, ...patch });
    const db = await this.maybeDb();
    if (db) {
      await db.put(STORE_SETTINGS, next, "uiSettings");
      if (patch.localOnly != null) {
        await db.put(STORE_SETTINGS, next.localOnly, "localOnly");
      }
      return next;
    }

    if (canUseExtensionStorage()) {
      const payload: Record<string, unknown> = {
        [`${EXT_SETTING_PREFIX}uiSettings`]: next,
      };
      if (patch.localOnly != null) {
        payload[`${EXT_SETTING_PREFIX}localOnly`] = next.localOnly;
      }
      await browser.storage.local.set(payload);
    }

    return next;
  }

  async getWorkbenchDraft(): Promise<FlowWorkbenchDraft> {
    const db = await this.maybeDb();
    if (db) {
      const stored = await db.get(STORE_SETTINGS, "workbenchDraft");
      return normalizeWorkbenchDraft(stored);
    }

    if (canUseExtensionStorage()) {
      const stored = await browser.storage.local.get(`${EXT_SETTING_PREFIX}workbenchDraft`);
      return normalizeWorkbenchDraft(stored[`${EXT_SETTING_PREFIX}workbenchDraft`]);
    }

    return { ...DEFAULT_WORKBENCH_DRAFT };
  }

  async updateWorkbenchDraft(
    patch: Partial<FlowWorkbenchDraft>,
  ): Promise<FlowWorkbenchDraft> {
    const current = await this.getWorkbenchDraft();
    const next = normalizeWorkbenchDraft({ ...current, ...patch });
    const db = await this.maybeDb();
    if (db) {
      await db.put(STORE_SETTINGS, next, "workbenchDraft");
      return next;
    }

    if (canUseExtensionStorage()) {
      await browser.storage.local.set({
        [`${EXT_SETTING_PREFIX}workbenchDraft`]: next,
      });
    }

    return next;
  }

  async hasPack(packKey: string): Promise<boolean> {
    const manifest = await this.getPackManifest(packKey);
    if (!manifest) {
      return false;
    }

    const status = await this.getPackStatus(manifest);
    return status.state === "ready";
  }

  async getPackManifest(packKey: string): Promise<BrowserPackManifest | null> {
    const db = await this.maybeDb();
    if (db) {
      const record = (await db.get(
        STORE_MANIFESTS,
        packKey,
      )) as StoredManifestRecord | undefined;
      return record?.manifest ?? null;
    }

    if (!canUseExtensionStorage()) {
      return null;
    }

    const result = await browser.storage.local.get(extManifestKey(packKey));
    const record = result[extManifestKey(packKey)] as StoredManifestRecord | undefined;
    return record?.manifest ?? null;
  }

  async putPackManifest(pack: BrowserPackManifest) {
    const record = {
      key: pack.packKey,
      manifest: pack,
      updatedAt: Date.now(),
    } satisfies StoredManifestRecord;

    const db = await this.maybeDb();
    if (db) {
      await db.put(STORE_MANIFESTS, record);
      return;
    }

    if (canUseExtensionStorage()) {
      await browser.storage.local.set({
        [extManifestKey(pack.packKey)]: record,
      });
    }
  }

  async readPackFile(packKey: string, filePath: string): Promise<StoredFileRecord | null> {
    const metadata = await this.readFileMetadata(packKey, filePath);
    const opfsRecord = await this.readOpfsFile(packKey, filePath);
    if (opfsRecord) {
      return {
        ...opfsRecord,
        contentType: metadata?.contentType ?? opfsRecord.contentType,
        sha256: metadata?.sha256 ?? opfsRecord.sha256,
        sizeBytes: metadata?.sizeBytes ?? opfsRecord.sizeBytes,
      };
    }

    const db = await this.maybeDb();
    if (db) {
      const record = (await db.get(STORE_FILES, fileKey(packKey, filePath))) as
        | StoredFileRecord
        | undefined;
      return record ?? null;
    }

    if (!canUseExtensionStorage()) {
      return null;
    }

    const result = await browser.storage.local.get(extFileKey(packKey, filePath));
    const record = result[extFileKey(packKey, filePath)] as StoredFileRecord | undefined;
    if (!record) {
      return null;
    }

    return {
      ...record,
      bytes: record.base64 ? fromBase64(record.base64) : undefined,
    };
  }

  async writePackFile(
    packKey: string,
    filePath: string,
    bytes: ArrayBuffer,
    contentType = guessContentType(filePath),
  ) {
    const sha256 = await sha256Hex(bytes);
    const record = {
      key: fileKey(packKey, filePath),
      contentType,
      updatedAt: Date.now(),
      sizeBytes: bytes.byteLength,
      sha256,
      opfs: false,
    };

    const wroteToOpfs = await this.writeOpfsFile(packKey, filePath, bytes);
    const db = await this.maybeDb();
    if (db) {
      await db.put(STORE_FILES, {
        ...record,
        bytes: wroteToOpfs ? undefined : bytes,
        opfs: wroteToOpfs,
      } satisfies StoredFileRecord);
      return;
    }

    if (canUseExtensionStorage()) {
      await browser.storage.local.set({
        [extFileKey(packKey, filePath)]: {
          ...record,
          base64: wroteToOpfs ? undefined : toBase64(bytes),
          opfs: wroteToOpfs,
        } satisfies StoredFileRecord,
      });
    }
  }

  async ensurePackDownloaded(
    pack: BrowserPackManifest,
    onProgress?: (status: string) => void,
  ) {
    await this.putPackManifest(pack);

    const initialStatus = await this.getPackStatus(pack);
    if (initialStatus.state === "ready") {
      onProgress?.(`Pack "${pack.displayName}" is already cached and verified.`);
      return;
    }

    let completed = initialStatus.filesReady;
    for (const file of pack.files) {
      const verification = await this.verifyPackFile(pack.packKey, file);
      if (verification === "ready") {
        onProgress?.(
          `Verified cached file ${completed}/${pack.files.length}: ${file.path}`,
        );
        continue;
      }

      onProgress?.(
        `Downloading ${pack.displayName}: ${completed + 1}/${pack.files.length} ${file.path}`,
      );
      const response = await fetch(file.sourceUrl, { cache: "no-store" });
      if (!response.ok) {
        throw new Error(`Failed to download ${file.sourceUrl}: ${response.status}`);
      }
      const bytes = await response.arrayBuffer();
      await this.writePackFile(
        pack.packKey,
        file.path,
        bytes,
        file.contentType ?? guessContentType(file.path),
      );

      const postWrite = await this.verifyPackFile(pack.packKey, file);
      if (postWrite !== "ready") {
        await this.removePackFile(pack.packKey, file.path);
        throw new Error(`Integrity verification failed for ${file.path}.`);
      }

      completed += 1;
    }

    const finalStatus = await this.getPackStatus(pack);
    if (finalStatus.state !== "ready") {
      throw new Error(
        `Pack "${pack.displayName}" is not fully verified after download (${finalStatus.state}).`,
      );
    }

    onProgress?.(`Pack "${pack.displayName}" is ready for local inference.`);
  }

  async getPackStatus(pack: BrowserPackManifest): Promise<BrowserPackStatus> {
    let filesReady = 0;
    let lastUpdatedAt: number | null = null;
    let sawMissing = false;
    let sawCorrupt = false;

    for (const file of pack.files) {
      const verification = await this.verifyPackFile(pack.packKey, file);
      const record = await this.readPackFile(pack.packKey, file.path);
      lastUpdatedAt = Math.max(lastUpdatedAt ?? 0, record?.updatedAt ?? 0) || lastUpdatedAt;

      if (verification === "ready") {
        filesReady += 1;
      } else if (verification === "missing") {
        sawMissing = true;
      } else {
        sawCorrupt = true;
      }
    }

    let state: BrowserPackStatus["state"] = "missing";
    if (filesReady === pack.files.length && pack.files.length > 0) {
      state = "ready";
    } else if (sawCorrupt) {
      state = "corrupt";
    } else if (filesReady > 0 || sawMissing) {
      state = filesReady === 0 ? "missing" : "partial";
    }

    return {
      packKey: pack.packKey,
      modelKey: pack.modelKey,
      displayName: pack.displayName,
      state,
      filesReady,
      filesTotal: pack.files.length,
      storageBackend: await this.backend(),
      lastUpdatedAt,
      lastError:
        state === "corrupt"
          ? "At least one cached file failed local integrity checks."
          : null,
    };
  }

  async listPackStatuses(catalog: BrowserPackManifest[]): Promise<BrowserPackStatus[]> {
    return Promise.all(catalog.map((pack) => this.getPackStatus(pack)));
  }

  async removePack(packKey: string) {
    const manifest = await this.getPackManifest(packKey);
    const knownFiles = manifest?.files ?? [];

    for (const file of knownFiles) {
      await this.removePackFile(packKey, file.path);
    }

    await this.removePackManifest(packKey);
  }

  private async maybeDb() {
    if (!canUseIndexedDb()) {
      return null;
    }

    return openDB(DB_NAME, DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains(STORE_FILES)) {
          db.createObjectStore(STORE_FILES, { keyPath: "key" });
        }
        if (!db.objectStoreNames.contains(STORE_MANIFESTS)) {
          db.createObjectStore(STORE_MANIFESTS, { keyPath: "key" });
        }
        if (!db.objectStoreNames.contains(STORE_SETTINGS)) {
          db.createObjectStore(STORE_SETTINGS);
        }
      },
    });
  }

  private async verifyPackFile(packKey: string, file: BrowserPackFile): Promise<VerifyState> {
    const record = await this.readPackFile(packKey, file.path);
    if (!record?.bytes) {
      return "missing";
    }

    if (record.bytes.byteLength === 0) {
      return "corrupt";
    }

    if (file.sizeBytes != null && file.sizeBytes !== record.bytes.byteLength) {
      return "corrupt";
    }

    const currentSha = file.sha256 ?? record.sha256;
    if (currentSha) {
      const digest = await sha256Hex(record.bytes);
      if (!digest || digest.toLowerCase() !== currentSha.toLowerCase()) {
        return "corrupt";
      }
    }

    return "ready";
  }

  private async readFileMetadata(packKey: string, filePath: string): Promise<StoredFileRecord | null> {
    const db = await this.maybeDb();
    if (db) {
      const record = (await db.get(STORE_FILES, fileKey(packKey, filePath))) as
        | StoredFileRecord
        | undefined;
      return record ?? null;
    }

    if (!canUseExtensionStorage()) {
      return null;
    }

    const result = await browser.storage.local.get(extFileKey(packKey, filePath));
    return (result[extFileKey(packKey, filePath)] as StoredFileRecord | undefined) ?? null;
  }

  private async removePackManifest(packKey: string) {
    const db = await this.maybeDb();
    if (db) {
      await db.delete(STORE_MANIFESTS, packKey);
      return;
    }

    if (canUseExtensionStorage()) {
      await browser.storage.local.remove(extManifestKey(packKey));
    }
  }

  private async removePackFile(packKey: string, filePath: string) {
    await this.removeOpfsFile(packKey, filePath);

    const db = await this.maybeDb();
    if (db) {
      await db.delete(STORE_FILES, fileKey(packKey, filePath));
      return;
    }

    if (canUseExtensionStorage()) {
      await browser.storage.local.remove(extFileKey(packKey, filePath));
    }
  }

  private async opfsRoot() {
    if (
      typeof navigator === "undefined" ||
      !navigator.storage ||
      typeof navigator.storage.getDirectory !== "function"
    ) {
      return null;
    }

    return navigator.storage.getDirectory();
  }

  private async writeOpfsFile(
    packKey: string,
    filePath: string,
    bytes: ArrayBuffer,
  ): Promise<boolean> {
    const root = await this.opfsRoot();
    if (!root) {
      return false;
    }

    const segments = ["packs", packKey, ...filePath.split("/")];
    let directory = root;
    for (const segment of segments.slice(0, -1)) {
      directory = await directory.getDirectoryHandle(segment, { create: true });
    }

    const fileHandle = await directory.getFileHandle(segments.at(-1)!, {
      create: true,
    });
    const writable = await fileHandle.createWritable();
    await writable.write(bytes);
    await writable.close();
    return true;
  }

  private async readOpfsFile(
    packKey: string,
    filePath: string,
  ): Promise<StoredFileRecord | null> {
    const root = await this.opfsRoot();
    if (!root) {
      return null;
    }

    try {
      const segments = ["packs", packKey, ...filePath.split("/")];
      let directory = root;
      for (const segment of segments.slice(0, -1)) {
        directory = await directory.getDirectoryHandle(segment);
      }

      const handle = await directory.getFileHandle(segments.at(-1)!);
      const file = await handle.getFile();
      return {
        key: fileKey(packKey, filePath),
        bytes: await file.arrayBuffer(),
        contentType: guessContentType(filePath),
        updatedAt: file.lastModified,
        sizeBytes: file.size,
        sha256: null,
        opfs: true,
      };
    } catch {
      return null;
    }
  }

  private async removeOpfsFile(packKey: string, filePath: string) {
    const root = await this.opfsRoot();
    if (!root) {
      return;
    }

    try {
      const segments = ["packs", packKey, ...filePath.split("/")];
      let directory = root;
      for (const segment of segments.slice(0, -1)) {
        directory = await directory.getDirectoryHandle(segment);
      }

      await directory.removeEntry(segments.at(-1)!);
    } catch {
      // ignore missing OPFS files
    }
  }
}
