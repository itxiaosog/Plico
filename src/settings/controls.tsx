/**
 * 设置页的通用控件。
 *
 * 全部是受控组件：值来自 store，改动通过回调上抛。数字输入是例外 ——
 * 它在内部保留一份草稿字符串，只在失焦 / 回车时提交。否则用户输入过程中
 * 每敲一个字符就写一次库，而且「清空再输」这种中间态会被钳制成一个荒唐的值。
 */
import { useEffect, useState, type ReactNode } from "react";
import {
  PixelButton,
  PixelInput,
  PixelSegmented,
  PixelSwitch,
} from "@pxlkit/ui-kit";

export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="pl-set-section">
      <h2 className="pl-set-section__title">{title}</h2>
      <div className="pl-set-card">{children}</div>
    </section>
  );
}

export function Row({
  label,
  hint,
  children,
  stacked = false,
}: {
  label: string;
  hint?: ReactNode;
  children?: ReactNode;
  /** 内容需要占满整行（比如规则列表）时用。 */
  stacked?: boolean;
}) {
  return (
    <div className={`pl-set-row${stacked ? " is-stacked" : ""}`}>
      <div className="pl-set-row__label">
        <span>{label}</span>
        {hint && <span className="pl-set-row__hint">{hint}</span>}
      </div>
      {children && <div className="pl-set-row__control">{children}</div>}
    </div>
  );
}

export function Switch({
  checked,
  disabled,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange(next: boolean): void;
}) {
  // 行内文案已经由 Row 的 label 承担，这里的 label 只为满足必填签名
  return (
    <PixelSwitch
      label=""
      checked={checked}
      disabled={disabled}
      onChange={onChange}
    />
  );
}

export interface Option<T extends string> {
  value: T;
  label: string;
}

export function Segmented<T extends string>({
  value,
  options,
  disabled,
  onChange,
}: {
  value: T;
  options: Option<T>[];
  disabled?: boolean;
  onChange(next: T): void;
}) {
  return (
    <PixelSegmented
      value={value}
      options={options}
      disabled={disabled}
      onChange={(next) => onChange(next as T)}
    />
  );
}

export function NumberField({
  value,
  min,
  max,
  unit,
  disabled,
  onCommit,
}: {
  value: number;
  min: number;
  max: number;
  unit?: string;
  disabled?: boolean;
  onCommit(next: number): void;
}) {
  const [draft, setDraft] = useState(String(value));

  // 外部值变了（比如后端钳制后回写）就同步回草稿
  useEffect(() => {
    setDraft(String(value));
  }, [value]);

  const commit = () => {
    const parsed = Number(draft);
    if (!Number.isFinite(parsed)) {
      setDraft(String(value));
      return;
    }
    const clamped = Math.min(max, Math.max(min, Math.round(parsed)));
    setDraft(String(clamped));
    if (clamped !== value) onCommit(clamped);
  };

  return (
    <PixelInput
      type="text"
      inputMode="numeric"
      size="sm"
      value={draft}
      disabled={disabled}
      suffix={unit}
      style={{ width: 108 }}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          e.currentTarget.blur();
        } else if (e.key === "Escape") {
          setDraft(String(value));
          e.currentTarget.blur();
        }
      }}
    />
  );
}

export function Button({
  children,
  onClick,
  variant = "default",
  disabled,
}: {
  children: ReactNode;
  onClick?(): void;
  variant?: "default" | "primary" | "danger";
  disabled?: boolean;
}) {
  // 旧 variant → pxlkit (variant, tone)。pxlkit 没有 "primary" 色调，
  // 肯定动作用它的招牌 solid green；danger 维持描边款，别把确认区搞得太吵。
  const look =
    variant === "primary"
      ? ({ variant: "solid", tone: "green" } as const)
      : variant === "danger"
        ? ({ variant: "outline", tone: "red" } as const)
        : ({ variant: "outline", tone: "neutral" } as const);
  return (
    <PixelButton {...look} size="sm" disabled={disabled} onClick={onClick}>
      {children}
    </PixelButton>
  );
}

/** 顶部的错误条。设置写入失败时把后端原话摆出来，不做二次包装。 */
export function ErrorBanner({ text, onClose }: { text: string; onClose(): void }) {
  return (
    <div className="pl-set-error" role="alert">
      <span>{text}</span>
      <button type="button" className="pl-set-error__close" onClick={onClose} aria-label="关闭">
        ×
      </button>
    </div>
  );
}
