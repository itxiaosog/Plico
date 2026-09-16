import { create } from "zustand";

import * as api from "../api";
import type { AppSettings, Rule, RuleKind } from "../types";

/**
 * 设置窗口的状态。
 *
 * 写入策略是「以返回值为准」：`update_settings` 返回的是后端落库后的权威设置
 * （越界值已被钳制、无效热键已被拒绝），所以每次写完都拿它覆盖本地状态，
 * 不在前端做乐观更新 —— 否则界面会显示一个库里并不存在的值。
 */
interface SettingsState {
  settings: AppSettings | null;
  rules: Rule[];
  loading: boolean;
  /** 最近一次操作的错误，展示在内容区顶部。 */
  error: string | null;

  load(): Promise<void>;
  /** 返回是否成功。失败原因已写进 `error`。 */
  patch(patch: Partial<AppSettings>): Promise<boolean>;

  addRule(kind: RuleKind, value: string): Promise<boolean>;
  deleteRule(id: number): Promise<boolean>;
  setRuleEnabled(id: number, enabled: boolean): Promise<boolean>;

  clearError(): void;
}

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: null,
  rules: [],
  loading: false,
  error: null,

  async load() {
    set({ loading: true });
    try {
      const [settings, rules] = await Promise.all([api.getSettings(), api.listRules()]);
      set({ settings, rules, loading: false });
    } catch (e) {
      set({ loading: false, error: message(e) });
    }
  },

  async patch(patch) {
    try {
      const settings = await api.updateSettings(patch);
      set({ settings, error: null });
      return true;
    } catch (e) {
      set({ error: message(e) });
      return false;
    }
  },

  async addRule(kind, value) {
    try {
      set({ rules: await api.addRule(kind, value), error: null });
      return true;
    } catch (e) {
      set({ error: message(e) });
      return false;
    }
  },

  async deleteRule(id) {
    try {
      set({ rules: await api.deleteRule(id), error: null });
      return true;
    } catch (e) {
      set({ error: message(e) });
      return false;
    }
  },

  async setRuleEnabled(id, enabled) {
    try {
      set({ rules: await api.setRuleEnabled(id, enabled), error: null });
      return true;
    } catch (e) {
      set({ error: message(e) });
      return false;
    }
  },

  clearError() {
    set({ error: null });
  },
}));
