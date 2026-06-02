import type { ReactNode } from "react";
import { startDragging } from "../features/window/api";

type DragRegionProps = {
  children: ReactNode;
  onDragComplete?: () => void;
};

export function DragRegion({ children, onDragComplete }: DragRegionProps) {
  return (
    <div
      className="drag-region"
      onMouseDown={(event) => {
        if (event.button !== 0) {
          return;
        }

        void startDragging().finally(() => {
          window.setTimeout(() => onDragComplete?.(), 250);
        });
      }}
    >
      {children}
    </div>
  );
}
