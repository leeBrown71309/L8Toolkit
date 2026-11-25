import type { ReactNode } from "react";

interface ToggleProps {
  checked: boolean;
  label: string;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

export function Toggle({ checked, label, disabled = false, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      className="toggle"
      onClick={() => onChange(!checked)}
    >
      <span className="toggle__thumb" />
    </button>
  );
}

interface SegmentedOption<T extends string> {
  value: T;
  label: string;
}

interface SegmentedProps<T extends string> {
  label: string;
  value: T;
  options: SegmentedOption<T>[];
  onChange: (value: T) => void;
}

export function Segmented<T extends string>({ label, value, options, onChange }: SegmentedProps<T>) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          className="segmented__option"
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

interface SettingsGroupProps {
  title: string;
  children: ReactNode;
}

export function SettingsGroup({ title, children }: SettingsGroupProps) {
  return (
    <section className="settings-group" aria-label={title}>
      <h2 className="settings-group__title">{title}</h2>
      <div className="settings-group__rows">{children}</div>
    </section>
  );
}

interface SettingRowProps {
  label: string;
  description?: ReactNode;
  control?: ReactNode;
  children?: ReactNode;
  disabled?: boolean;
}

export function SettingRow({ label, description, control, children, disabled = false }: SettingRowProps) {
  return (
    <div className="setting-row" data-disabled={disabled}>
      <div className="setting-row__main">
        <div className="setting-row__text">
          <span className="setting-row__label">{label}</span>
          {description && <span className="setting-row__description">{description}</span>}
        </div>
        {control && <div className="setting-row__control">{control}</div>}
      </div>
      {children}
    </div>
  );
}

export function SectionHeader({ title, description }: { title: string; description: string }) {
  return (
    <header className="section-header">
      <h1 className="section-header__title">{title}</h1>
      <p className="section-header__description">{description}</p>
    </header>
  );
}
