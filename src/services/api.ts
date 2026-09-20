/**
 * The complete IPC surface, one function per Rust command.
 *
 * Components never call `invoke` directly — everything goes through here so
 * command names and argument shapes live in one file.
 */

import { invoke } from './ipc';
import type {
  Prompt,
  AddAttachmentsResult,
  AppInfo,
  ClaudeCodeStatus,
  Attachment,
  Conversation,
  ConversationSummary,
  ExportFormat,
  ExportResult,
  ListScope,
  Message,
  ModelListResult,
  Project,
  ProjectInput,
  ProjectSummary,
  SearchHit,
  Settings,
  StorageStats,
  UsageTotals,
} from '@/types';

// --- conversations --------------------------------------------------------

export const listConversations = (
  scope: ListScope = 'active',
  projectId?: string | null,
  limit = 200,
  offset = 0,
) =>
  invoke<ConversationSummary[]>('list_conversations', {
    scope,
    projectId: projectId ?? null,
    limit,
    offset,
  });

export const getConversation = (id: string) =>
  invoke<Conversation>('get_conversation', { id });

export const createConversation = (input: {
  title?: string | null;
  model: string;
  projectId?: string | null;
  systemPrompt?: string | null;
}) =>
  invoke<Conversation>('create_conversation', {
    input: {
      title: input.title ?? null,
      model: input.model,
      projectId: input.projectId ?? null,
      systemPrompt: input.systemPrompt ?? null,
    },
  });

export const getMessages = (conversationId: string) =>
  invoke<Message[]>('get_messages', { conversationId });

export const renameConversation = (id: string, title: string) =>
  invoke<void>('rename_conversation', { id, title });

export const setConversationModel = (id: string, model: string) =>
  invoke<void>('set_conversation_model', { id, model });

export const setConversationPinned = (id: string, pinned: boolean) =>
  invoke<void>('set_conversation_pinned', { id, pinned });

export const setConversationArchived = (id: string, archived: boolean) =>
  invoke<void>('set_conversation_archived', { id, archived });

export const setConversationProject = (id: string, projectId: string | null) =>
  invoke<void>('set_conversation_project', { id, projectId });

export const setConversationSystemPrompt = (id: string, prompt: string | null) =>
  invoke<void>('set_conversation_system_prompt', { id, prompt });

export const trashConversation = (id: string) =>
  invoke<void>('trash_conversation', { id });

export const restoreConversation = (id: string) =>
  invoke<void>('restore_conversation', { id });

export const deleteConversationPermanently = (id: string) =>
  invoke<void>('delete_conversation_permanently', { id });

export const emptyTrash = () => invoke<number>('empty_trash');

export const duplicateConversation = (id: string) =>
  invoke<Conversation>('duplicate_conversation', { id });

export const branchConversation = (messageId: string) =>
  invoke<Conversation>('branch_conversation', { messageId });

export const conversationUsage = (id: string) =>
  invoke<UsageTotals>('conversation_usage', { id });

export const exportConversation = (
  id: string,
  format: ExportFormat,
  includeMetadata = true,
) => invoke<ExportResult>('export_conversation', { id, format, includeMetadata });

export const writeTextFile = (path: string, contents: string) =>
  invoke<void>('write_text_file', { path, contents });

export const deleteMessage = (id: string) => invoke<void>('delete_message', { id });

/** Rewrite a user turn, drop everything after it, and answer again. */
export const editAndResend = (conversationId: string, messageId: string, text: string) =>
  invoke<string>('edit_and_resend', { conversationId, messageId, text });

// --- chat -----------------------------------------------------------------

export const sendMessage = (
  conversationId: string,
  text: string,
  attachmentIds: string[] = [],
) => invoke<string>('send_message', { conversationId, text, attachmentIds });

export const retryMessage = (conversationId: string) =>
  invoke<string>('retry_message', { conversationId });

export const continueMessage = (conversationId: string) =>
  invoke<string>('continue_message', { conversationId });

export const stopGeneration = (conversationId: string) =>
  invoke<void>('stop_generation', { conversationId });

export const isGenerating = (conversationId: string) =>
  invoke<boolean>('is_generating', { conversationId });

// --- settings & models ----------------------------------------------------

export const getSettings = () => invoke<Settings>('get_settings');
export const setSetting = <K extends keyof Settings>(key: K, value: Settings[K]) =>
  invoke<void>('set_setting', { key, value });
export const setSettings = (values: Partial<Settings>) =>
  invoke<void>('set_settings', { values });
export const resetSetting = (key: keyof Settings) => invoke<void>('reset_setting', { key });
export const listModels = (refresh = false) =>
  invoke<ModelListResult>('list_models', { refresh });

// --- projects -------------------------------------------------------------

export const listProjects = (includeArchived = false) =>
  invoke<ProjectSummary[]>('list_projects', { includeArchived });
export const getProject = (id: string) => invoke<Project>('get_project', { id });
export const createProject = (input: ProjectInput) =>
  invoke<Project>('create_project', { input });
export const updateProject = (id: string, input: ProjectInput) =>
  invoke<Project>('update_project', { id, input });
export const setProjectArchived = (id: string, archived: boolean) =>
  invoke<void>('set_project_archived', { id, archived });
export const deleteProject = (id: string) => invoke<void>('delete_project', { id });

/** Claim a desktop-wide quick-chat shortcut; empty clears it. */
export const setQuickChatShortcut = (accelerator: string) =>
  invoke<void>('set_quick_chat_shortcut', { accelerator });

// --- prompt library -------------------------------------------------------

export const listPrompts = () => invoke<Prompt[]>('list_prompts');
export const createPrompt = (title: string, body: string) =>
  invoke<Prompt>('create_prompt', { title, body });
export const updatePrompt = (id: string, title: string, body: string) =>
  invoke<Prompt>('update_prompt', { id, title, body });
export const deletePrompt = (id: string) => invoke<void>('delete_prompt', { id });
export const markPromptUsed = (id: string) => invoke<void>('mark_prompt_used', { id });

// --- search ---------------------------------------------------------------

export const searchAll = (query: string, limit = 50) =>
  invoke<SearchHit[]>('search_all', { query, limit });
export const searchConversation = (conversationId: string, query: string, limit = 100) =>
  invoke<SearchHit[]>('search_conversation', { conversationId, query, limit });
export const rebuildSearchIndex = () => invoke<void>('rebuild_search_index');

// --- attachments ----------------------------------------------------------

export const addAttachments = (
  paths: string[],
  conversationId?: string | null,
  projectId?: string | null,
) =>
  invoke<AddAttachmentsResult>('add_attachments', {
    paths,
    conversationId: conversationId ?? null,
    projectId: projectId ?? null,
  });

export const addImageAttachment = (
  filename: string,
  mimeType: string,
  dataBase64: string,
  conversationId?: string | null,
) =>
  invoke<Attachment>('add_image_attachment', {
    filename,
    mimeType,
    dataBase64,
    conversationId: conversationId ?? null,
  });

export const removeAttachment = (id: string) => invoke<void>('remove_attachment', { id });
export const listAttachments = (conversationId: string) =>
  invoke<Attachment[]>('list_attachments', { conversationId });
export const readAttachmentDataUrl = (id: string) =>
  invoke<string>('read_attachment_data_url', { id });

// --- system ---------------------------------------------------------------

export const appInfo = () => invoke<AppInfo>('app_info');
/** Is the Claude Code CLI installed and runnable? */
export const claudeCodeStatus = () => invoke<ClaudeCodeStatus>('claude_code_status');
export const storageStats = () => invoke<StorageStats>('storage_stats');
export const backupDatabase = (path: string) => invoke<void>('backup_database', { path });
export const vacuumDatabase = () => invoke<void>('vacuum_database');
export const clearAllConversations = () => invoke<void>('clear_all_conversations');
export const exportAllData = () => invoke<string>('export_all_data');
export const pruneOrphanAttachments = () => invoke<number>('prune_orphan_attachments');
