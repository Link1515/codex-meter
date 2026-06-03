import type { ReactNode } from "react";
import { startDragging } from "../features/window/api";

type DragRegionProps = {
  children: ReactNode;
  className?: string;
  element?: "div" | "main";
  onDragComplete?: () => void;
};

const NON_DRAG_SELECTOR = "button, input, select, textarea, [role='button'], [data-no-drag]";

export function DragRegion({ children, className, element: Tag = "div", onDragComplete }: DragRegionProps) {
  return (
    <Tag
      className={["drag-region", className].filter(Boolean).join(" ")}
      onMouseDown={(event) => {
        if (event.button !== 0) {
          return;
        }

        if (event.target instanceof Element && event.target.closest(NON_DRAG_SELECTOR)) {
          return;
        }

        void startDragging().finally(() => {
          window.setTimeout(() => onDragComplete?.(), 250);
        });
      }}
    >
      {children}
    </Tag>
  );
}
