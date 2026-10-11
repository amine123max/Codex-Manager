import { invoke, invokeFirst } from "./transport";
import {
  AccountManagerStatus,
  ApiKeyOwner,
  AppSettings,
  BackgroundTaskSettings,
  AppSessionResult,
  AppRole,
  AppPermission,
  AppUser,
  AppWallet,
  CodexLatestVersionInfo,
  ModelGroup,
  ModelGroupListResult,
  ModelGroupModel,
  UserModelGroup,
} from "../../types";
import { readBillingModeLock } from "./billing-mode-lock";
import { normalizeAppSettings } from "./normalize";
import {
  readUpdateActionResult,
  readUpdateCheckResult,
  readUpdatePrepareResult,
  readUpdateStatusResult,
  UpdateActionResult,
  UpdateCheckResult,
  UpdatePrepareResult,
  UpdateStatusResult,
} from "./app-updates";
import {
  GatewayConcurrencyRecommendation,
  readGatewayConcurrencyRecommendation,
} from "./gateway-settings";

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function asString(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function asNumber(value: unknown, fallback = 0): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function asBoolean(value: unknown, fallback = false): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function readWallet(value: unknown): AppWallet | null {
  const source = asRecord(value);
  const id = asString(source.id);
  if (!id) return null;
  return {
    id,
    ownerKind: asString(source.ownerKind),
    ownerId: asString(source.ownerId),
    balanceCreditMicros: asNumber(source.balanceCreditMicros),
    frozenCreditMicros: asNumber(source.frozenCreditMicros),
    availableCreditMicros: asNumber(source.availableCreditMicros),
    status: asString(source.status),
    createdAt: asNumber(source.createdAt),
    updatedAt: asNumber(source.updatedAt),
  };
}

function readAppUser(value: unknown): AppUser {
  const source = asRecord(value);
  return {
    id: asString(source.id),
    username: asString(source.username),
    displayName: asString(source.displayName) || null,
    role: asString(source.role) || "member",
    status: asString(source.status) || "active",
    createdAt: asNumber(source.createdAt),
    updatedAt: asNumber(source.updatedAt),
    lastLoginAt: asNumber(source.lastLoginAt) || null,
    wallet: readWallet(source.wallet),
  };
}

function readModelGroup(value: unknown): ModelGroup {
  const source = asRecord(value);
  return {
    id: asString(source.id),
    name: asString(source.name),
    description: asString(source.description) || null,
    status: asString(source.status) || "active",
    sort: asNumber(source.sort),
    isDefault: asBoolean(source.isDefault),
    rateMultiplierMillis: asNumber(source.rateMultiplierMillis, 1000),
    createdAt: asNumber(source.createdAt),
    updatedAt: asNumber(source.updatedAt),
  };
}

function readModelGroupModel(value: unknown): ModelGroupModel {
  const source = asRecord(value);
  return {
    groupId: asString(source.groupId),
    platformModelSlug: asString(source.platformModelSlug),
    enabled: asBoolean(source.enabled, true),
    rateMultiplierMillis:
      typeof source.rateMultiplierMillis === "number" ? source.rateMultiplierMillis : null,
    billingModelSlug: asString(source.billingModelSlug) || null,
    note: asString(source.note) || null,
    createdAt: asNumber(source.createdAt),
    updatedAt: asNumber(source.updatedAt),
  };
}

function readUserModelGroup(value: unknown): UserModelGroup {
  const source = asRecord(value);
  return {
    userId: asString(source.userId),
    groupId: asString(source.groupId),
    status: asString(source.status) || "active",
    expiresAt: typeof source.expiresAt === "number" ? source.expiresAt : null,
    createdAt: asNumber(source.createdAt),
    updatedAt: asNumber(source.updatedAt),
  };
}

function readModelGroupList(value: unknown): ModelGroupListResult {
  const source = asRecord(value);
  return {
    groups: Array.isArray(source.groups) ? source.groups.map(readModelGroup) : [],
    models: Array.isArray(source.models) ? source.models.map(readModelGroupModel) : [],
    userAssignments: Array.isArray(source.userAssignments)
      ? source.userAssignments.map(readUserModelGroup)
      : [],
  };
}

function readApiKeyOwner(value: unknown): ApiKeyOwner {
  const source = asRecord(value);
  return {
    keyId: asString(source.keyId),
    ownerKind: asString(source.ownerKind),
    ownerUserId: asString(source.ownerUserId) || null,
    projectId: asString(source.projectId) || null,
    updatedAt: asNumber(source.updatedAt),
  };
}

function readAccountManagerStatus(value: unknown): AccountManagerStatus {
  const source = asRecord(value);
  return {
    mode: asString(source.mode) || "none",
    modeOptions: Array.isArray(source.modeOptions)
      ? source.modeOptions.map((item) => asString(item)).filter(Boolean)
      : ["none", "password", "accounts"],
    passwordConfigured: asBoolean(source.passwordConfigured),
    appUsersConfigured: asBoolean(source.appUsersConfigured),
    appUserCount: asNumber(source.appUserCount),
    activeAdminCount: asNumber(source.activeAdminCount),
    distributionEnabled: asBoolean(source.distributionEnabled),
    billingModeLock: readBillingModeLock(source.billingModeLock),
  };
}

function readAppRole(value: unknown): AppRole {
  const role = asString(value);
  if (role === "admin" || role === "member" || role === "system_admin") {
    return role;
  }
  return "system_admin";
}

function readPermissions(value: unknown): AppPermission[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => asString(item))
    .filter((item): item is AppPermission =>
      [
        "system:admin",
        "apikey:self",
        "requestlog:self",
        "models:read",
        "profile:self",
      ].includes(item),
    );
}

function readAppSession(value: unknown): AppSessionResult {
  const source = asRecord(value);
  const currentUser = source.currentUser ? readAppUser(source.currentUser) : null;
  return {
    mode: asString(source.mode) || "none",
    currentUser,
    role: readAppRole(source.role),
    permissions: readPermissions(source.permissions),
    distributionEnabled: asBoolean(source.distributionEnabled),
    billingModeLock: readBillingModeLock(source.billingModeLock),
  };
}

export type AppSettingsPatch = Omit<Partial<AppSettings>, "backgroundTasks"> & {
  backgroundTasks?: Partial<BackgroundTaskSettings>;
};

export const appClient = {
  async getSettings(): Promise<AppSettings> {
    const result = await invoke<unknown>("app_settings_get");
    return normalizeAppSettings(result);
  },
  async setSettings(patch: AppSettingsPatch): Promise<AppSettings> {
    const result = await invoke<unknown>("app_settings_set", { patch });
    return normalizeAppSettings(result);
  },
  async getGatewayConcurrencyRecommendation(): Promise<GatewayConcurrencyRecommendation> {
    const result = await invoke<unknown>("service_gateway_concurrency_recommend_get");
    return readGatewayConcurrencyRecommendation(result);
  },
  async getAccountManagerStatus(): Promise<AccountManagerStatus> {
    const result = await invoke<unknown>("service_account_manager_status");
    return readAccountManagerStatus(result);
  },
  async getCurrentSession(): Promise<AppSessionResult> {
    const result = await invoke<unknown>("service_account_manager_session_current");
    return readAppSession(result);
  },
  async updateProfile(payload: {
    displayName?: string | null;
  }): Promise<AppUser> {
    const result = await invoke<unknown>("service_account_manager_profile_update", payload);
    return readAppUser(result);
  },
  async changePassword(payload: {
    currentPassword: string;
    newPassword: string;
  }): Promise<void> {
    await invoke<unknown>("service_account_manager_password_change", payload);
  },
  async listAppUsers(): Promise<AppUser[]> {
    const result = await invoke<unknown>("service_account_manager_users_list");
    return Array.isArray(result) ? result.map(readAppUser) : [];
  },
  async createAppUser(payload: {
    username: string;
    password: string;
    displayName?: string | null;
    role?: string | null;
    initialBalanceCreditMicros?: number | null;
  }): Promise<AppUser> {
    const result = await invoke<unknown>("service_account_manager_user_create", {
      payload,
    });
    return readAppUser(result);
  },
  async updateAppUser(payload: {
    id: string;
    displayName?: string | null;
    role?: string | null;
    status?: string | null;
    password?: string | null;
  }): Promise<AppUser> {
    const result = await invoke<unknown>("service_account_manager_user_update", {
      payload,
    });
    return readAppUser(result);
  },
  async deleteAppUser(id: string): Promise<void> {
    await invoke<unknown>("service_account_manager_user_delete", { id });
  },
  async topUpWallet(payload: {
    ownerKind: string;
    ownerId: string;
    amountCreditMicros: number;
    note?: string | null;
  }): Promise<AppWallet | null> {
    const result = await invoke<unknown>("service_account_manager_wallet_top_up", payload);
    return readWallet(result);
  },
  async setWalletAvailable(payload: {
    ownerKind: string;
    ownerId: string;
    availableCreditMicros: number;
    note?: string | null;
  }): Promise<AppWallet | null> {
    const result = await invoke<unknown>(
      "service_account_manager_wallet_set_available",
      payload
    );
    return readWallet(result);
  },
  async listApiKeyOwners(): Promise<ApiKeyOwner[]> {
    const result = await invoke<unknown>("service_account_manager_api_key_owners_list");
    return Array.isArray(result) ? result.map(readApiKeyOwner) : [];
  },
  async setApiKeyOwner(payload: {
    keyId: string;
    ownerKind: string;
    ownerUserId?: string | null;
    projectId?: string | null;
  }): Promise<ApiKeyOwner> {
    const result = await invoke<unknown>(
      "service_account_manager_api_key_owner_set",
      payload
    );
    return readApiKeyOwner(result);
  },
  async listModelGroups(): Promise<ModelGroupListResult> {
    const result = await invoke<unknown>("service_model_groups_list");
    return readModelGroupList(result);
  },
  async saveModelGroup(payload: {
    id?: string | null;
    name: string;
    description?: string | null;
    status?: string | null;
    sort?: number | null;
    isDefault?: boolean | null;
    rateMultiplierMillis?: number | null;
  }): Promise<ModelGroup> {
    const result = await invoke<unknown>("service_model_group_save", payload);
    return readModelGroup(result);
  },
  async deleteModelGroup(id: string): Promise<ModelGroupListResult> {
    const result = await invoke<unknown>("service_model_group_delete", { id });
    return readModelGroupList(result);
  },
  async setModelGroupModels(payload: {
    groupId: string;
    models: Array<{
      platformModelSlug: string;
      enabled?: boolean | null;
      rateMultiplierMillis?: number | null;
      billingModelSlug?: string | null;
      note?: string | null;
    }>;
  }): Promise<ModelGroupListResult> {
    const result = await invoke<unknown>("service_model_group_models_set", payload);
    return readModelGroupList(result);
  },
  async setModelGroupUsers(payload: {
    groupId: string;
    userIds: string[];
  }): Promise<ModelGroupListResult> {
    const result = await invoke<unknown>("service_model_group_users_set", payload);
    return readModelGroupList(result);
  },
  getCodexLatestVersion: () =>
    invoke<CodexLatestVersionInfo>("service_gateway_codex_latest_version_get"),

  getCloseToTray: () => invoke<boolean>("app_close_to_tray_on_close_get"),
  setCloseToTray: (enabled: boolean) =>
    invoke("app_close_to_tray_on_close_set", { enabled }),

  openInBrowser: (url: string) => invoke("open_in_browser", { url }),
  openExternalUrl: (url: string) => invoke("open_external_url", { url }),
  openInFileManager: (path: string) => invoke("open_in_file_manager", { path }),
  showMainWindow: () => invoke("app_show_main_window"),
  openUpdateLogsDir: (assetPath?: string) =>
    invoke("app_update_open_logs_dir", { assetPath: assetPath || null }),

  async checkUpdate(): Promise<UpdateCheckResult> {
    const result = await invokeFirst<unknown>(
      ["app_update_check", "update_check", "check_update"],
      {}
    );
    return readUpdateCheckResult(result);
  },
  async prepareUpdate(
    payload: Record<string, unknown> = {}
  ): Promise<UpdatePrepareResult> {
    const result = await invokeFirst<unknown>(
      ["app_update_prepare", "update_download", "download_update"],
      payload
    );
    return readUpdatePrepareResult(result);
  },
  async launchInstaller(
    payload: Record<string, unknown> = {}
  ): Promise<UpdateActionResult> {
    const result = await invokeFirst<unknown>(
      ["app_update_launch_installer", "update_install", "install_update"],
      payload
    );
    return readUpdateActionResult(result);
  },
  async applyUpdatePortable(
    payload: Record<string, unknown> = {}
  ): Promise<UpdateActionResult> {
    const result = await invokeFirst<unknown>(
      ["app_update_apply_portable", "update_restart", "restart_update"],
      payload
    );
    return readUpdateActionResult(result);
  },
  async getStatus(): Promise<UpdateStatusResult> {
    const result = await invokeFirst<unknown>(["app_update_status", "update_status"], {});
    return readUpdateStatusResult(result);
  },
};
