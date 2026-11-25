export interface KeyHint {
  keys: string[];
  label: string;
}

export function Keycap({ children }: { children: string }) {
  return <kbd className="keycap">{children}</kbd>;
}

export function KeyHintView({ hint }: { hint: KeyHint }) {
  return (
    <span className="key-hint">
      <span className="key-hint__keys">
        {hint.keys.map((key) => (
          <Keycap key={key}>{key}</Keycap>
        ))}
      </span>
      {hint.label}
    </span>
  );
}
