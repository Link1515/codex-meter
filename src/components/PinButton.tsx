import { Pin } from "lucide-react";

type PinButtonProps = {
  isPinned: boolean;
  isBusy?: boolean;
  onToggle: () => void;
};

export function PinButton({ isPinned, isBusy = false, onToggle }: PinButtonProps) {
  return (
    <button
      className={`icon-button ${isPinned ? "icon-button-active" : ""}`}
      type="button"
      aria-label={isPinned ? "Always on top enabled" : "Always on top disabled"}
      title={isPinned ? "Always on top enabled" : "Always on top disabled"}
      disabled={isBusy}
      onClick={onToggle}
    >
      <Pin size={16} aria-hidden="true" />
    </button>
  );
}
