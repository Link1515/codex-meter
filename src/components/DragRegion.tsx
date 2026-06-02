import type { ReactNode } from "react";
import { startDragging } from "../features/window/api";

type DragRegionProps = {
  children: ReactNode;
};

export function DragRegion({ children }: DragRegionProps) {
  return (
    <div
      className="drag-region"
      onMouseDown={(event) => {
        if (event.button !== 0) {
          return;
        }

        void startDragging();
      }}
    >
      {children}
    </div>
  );
}
