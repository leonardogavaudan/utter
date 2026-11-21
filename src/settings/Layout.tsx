import { type ReactNode, useId } from "react";

export type ControlProps = { id: string; "aria-labelledby": string; "aria-describedby"?: string };

export function Section({
  title,
  description,
  action,
  children,
}: {
  title: string;
  description?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="section">
      <header className="section-header">
        <div>
          <h2>{title}</h2>
          {description && <p className="section-description">{description}</p>}
        </div>
        {action}
      </header>
      <div className="section-body">{children}</div>
    </section>
  );
}

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: ReactNode;
  children: (control: ControlProps) => ReactNode;
}) {
  const id = useId();
  const labelId = `${id}-label`;
  const hintId = `${id}-hint`;
  return (
    <div className="field">
      <label className="field-label" id={labelId} htmlFor={id}>
        {label}
      </label>
      <div className="field-control">
        {children({ id, "aria-labelledby": labelId, "aria-describedby": hint ? hintId : undefined })}
        {hint && (
          <span className="field-hint" id={hintId}>
            {hint}
          </span>
        )}
      </div>
    </div>
  );
}

export function Switch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className="switch"
      onClick={() => onChange(!checked)}
    >
      <span className="switch-thumb" />
    </button>
  );
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  control,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  control: ControlProps;
}) {
  return (
    <fieldset className="segmented" {...control}>
      {options.map((option) => (
        <label key={option.value}>
          <input
            type="radio"
            name={control.id}
            value={option.value}
            checked={value === option.value}
            onChange={() => onChange(option.value)}
          />
          {option.label}
        </label>
      ))}
    </fieldset>
  );
}
