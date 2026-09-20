/**
 * Types mirroring the Rust IPC surface.
 *
 * Kept hand-written rather than generated so the shapes the UI actually needs
 * stay readable; `src/services/ipc.ts` is the single place they are applied.
 */

export type Role = 'user' | 'assistant' | 'system';

export type MessageStatus =
  | 'pending'
  | 'streaming'
  | 'complete'
  | 'interrupted'
  | 'error';

export type AttachmentKind = 'image' | 'document' | 'text';

export type ListScope = 'active' | 'archived' | 'trash';

export interface Conversation {
  id: string;
  title: string;
  titleLocked: boolean;
  provider: string;
  providerConversationId: string | null;
  model: string;
  systemPrompt: string | null;
  projectId: string | null;
  pinned: boolean;
  archived: boolean;
  deletedAt: number | null;
  branchedFromMessageId: string | null;
  createdAt: number;
  updatedAt: number;
  lastMessageAt: number | null;
  metadata: Record<string, unknown>;
}

export interface ConversationSummary extends Conversation {
  messageCount: number;
  preview: string | null;
  projectName: string | null;
}

export interface Attachment {
  id: string;
  conversationId: string | null;
  messageId: string | null;
  projectId: string | null;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  kind: AttachmentKind;
  storagePath: string;
  sha256: string;
  createdAt: number;
}

export interface Message {
  id: string;
  conversationId: string;
  seq: number;
  role: Role;
  content: string;
  thinking: string | null;
  /** Reasoning tokens the provider estimated. Claude Code reports a count
   *  but never the text, so this can be set while `thinking` is null. */
  thinkingTokens: number | null;
  status: MessageStatus;
  model: string | null;
  providerMessageId: string | null;
  stopReason: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
  cacheReadTokens: number | null;
  cacheWriteTokens: number | null;
  errorKind: string | null;
  errorMessage: string | null;
  createdAt: number;
  updatedAt: number;
  metadata: Record<string, unknown>;
  attachments: Attachment[];
}

export interface Project {
  id: string;
  name: string;
  description: string;
  instructions: string;
  workingDir: string | null;
  defaultModel: string | null;
  color: string | null;
  sortOrder: number;
  archived: boolean;
  createdAt: number;
  updatedAt: number;
  metadata: Record<string, unknown>;
}

export interface ProjectSummary extends Project {
  conversationCount: number;
}

export interface ProjectInput {
  name: string;
  description: string;
  instructions: string;
  workingDir: string | null;
  defaultModel: string | null;
  color: string | null;
}

export interface ModelInfo {
  id: string;
  displayName: string;
  createdAt: string | null;
  fromFallback: boolean;
}

export interface ModelListResult {
  models: ModelInfo[];
  /** The model the CLI reports as currently in use, if it said. */
  current: string | null;
  /** The effort the CLI reports as currently applied. */
  effort: string | null;
  /** True only when the CLI could not be asked and a fallback was used. */
  stale: boolean;
}

export interface SearchHit {
  conversationId: string;
  conversationTitle: string;
  messageId: string | null;
  kind: 'title' | 'message' | 'attachment' | 'project';
  role: Role | null;
  /** FTS5 snippet delimited with `<<` and `>>`. */
  snippet: string;
  createdAt: number;
  rank: number;
}

export interface UsageTotals {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  messageCount: number;
}

/** Whether this app's only backend — the Claude Code CLI — is usable. */
export interface ClaudeCodeStatus {
  installed: boolean;
  version: string | null;
  path: string | null;
}

export interface AppInfo {
  version: string;
  databasePath: string;
  dataDir: string;
  configDir: string;
  logDir: string;
  schemaVersion: number;
  devMode: boolean;
}

export interface StorageStats {
  databaseBytes: number;
  attachmentBytes: number;
  conversationCount: number;
  messageCount: number;
  attachmentCount: number;
  integrity: string;
}

export interface ErrorDetail {
  kind: string;
  message: string;
  status: number | null;
  requestId: string | null;
  retryable: boolean;
  retryAfterSecs: number | null;
}

/** The shape every failed `invoke` rejects with. */
export interface AppErrorPayload {
  kind: string;
  message: string;
  retryable: boolean;
  detail?: ErrorDetail;
}

export interface Rejection {
  filename: string;
  reason: string;
}

export interface AddAttachmentsResult {
  added: Attachment[];
  rejected: Rejection[];
}

export interface ExportResult {
  filename: string;
  content: string;
}

export type ExportFormat = 'markdown' | 'json' | 'text';

// --- streaming events -----------------------------------------------------

export interface StreamStartEvent {
  conversationId: string;
  messageId: string;
}

export interface StreamDeltaEvent {
  conversationId: string;
  messageId: string;
  text: string;
  channel: 'text' | 'thinking';
}

export interface ThinkingUpdateEvent {
  conversationId: string;
  messageId: string;
  tokens: number | null;
}

export interface StreamEndEvent {
  conversationId: string;
  messageId: string;
  status: MessageStatus;
  stopReason: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
  error: ErrorDetail | null;
}

// --- settings -------------------------------------------------------------

export type ThemePreference = 'system' | 'light' | 'dark';
export type EffortLevel = 'low' | 'medium' | 'high' | 'xhigh' | 'max';
export type SendKeyPreference = 'enter' | 'ctrlEnter';

export interface Settings {
  'claude.defaultModel': string | null;
  'claude.autoTitle': boolean;
  'claude.titleModel': string | null;
  /** `--effort` passed to the CLI; null leaves the CLI's own default. */
  'claude.effort': EffortLevel | null;
  /** Discovered from a session, not configured. Used for composer completion. */
  'claude.slashCommands': string[];
  /** Tools a session offered that this build does not disable. Should be []. */
  'claude.unexpectedTools': string[];

  'appearance.theme': ThemePreference;
  'appearance.fontSize': number;
  'appearance.compact': boolean;
  'appearance.codeWrap': boolean;
  'appearance.reducedMotion': boolean;

  'general.sendKey': SendKeyPreference;
  'general.restoreLastConversation': boolean;
  'general.trayEnabled': boolean;

  'notifications.enabled': boolean;
  'notifications.minDurationMs': number;

  'privacy.historyEnabled': boolean;

  'advanced.debugLogs': boolean;
  'advanced.trashRetentionDays': number;

  'ui.lastConversationId': string | null;
  'ui.sidebarCollapsed': boolean;
  'ui.onboarded': boolean;
}

export type SettingKey = keyof Settings;
