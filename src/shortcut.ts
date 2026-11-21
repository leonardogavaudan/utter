const MODIFIERS = ["Control", "Alt", "Shift", "Super"] as const;

const IS_MAC = navigator.userAgent.includes("Mac");

const MODIFIER_ALIASES: Record<string, (typeof MODIFIERS)[number]> = {
  control: "Control",
  ctrl: "Control",
  alt: "Alt",
  option: "Alt",
  shift: "Shift",
  super: "Super",
  cmd: "Super",
  command: "Super",
  meta: "Super",
  cmdorctrl: IS_MAC ? "Super" : "Control",
};

const MAC_SYMBOLS: Record<(typeof MODIFIERS)[number], string> = {
  Control: "⌃",
  Alt: "⌥",
  Shift: "⇧",
  Super: "⌘",
};

const OTHER_LABELS: Record<(typeof MODIFIERS)[number], string> = {
  Control: "Ctrl",
  Alt: "Alt",
  Shift: "Shift",
  Super: "Win",
};

const MODIFIER_CODES = new Set([
  "ShiftLeft",
  "ShiftRight",
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
  "Fn",
  "CapsLock",
]);

export function shortcutKeys(shortcut: string): string[] {
  return shortcut
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const modifier = MODIFIER_ALIASES[part.toLowerCase()];
      if (modifier) return IS_MAC ? MAC_SYMBOLS[modifier] : OTHER_LABELS[modifier];
      return keyLabel(part);
    });
}

function keyLabel(code: string): string {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (code === "Space") return "Space";
  return code.replace(/^Arrow/, "").replace(/^Numpad/, "Num ");
}

export type ShortcutCapture =
  | { kind: "pending" }
  | { kind: "cancelled" }
  | { kind: "invalid"; reason: string }
  | { kind: "captured"; shortcut: string };

export function captureShortcut(event: KeyboardEvent): ShortcutCapture {
  if (MODIFIER_CODES.has(event.code)) return { kind: "pending" };
  const modifiers = MODIFIERS.filter(
    (modifier) =>
      ({
        Control: event.ctrlKey,
        Alt: event.altKey,
        Shift: event.shiftKey,
        Super: event.metaKey,
      })[modifier],
  );
  if (event.code === "Escape" && modifiers.length === 0) return { kind: "cancelled" };
  const isFunctionKey = /^F\d{1,2}$/.test(event.code);
  if (modifiers.length === 0 && !isFunctionKey) {
    return { kind: "invalid", reason: "Add a modifier such as ⌥ or ⌃, or use a function key." };
  }
  return { kind: "captured", shortcut: [...modifiers, event.code].join("+") };
}
