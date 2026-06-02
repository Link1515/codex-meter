export type WindowPinState = {
  isPinned: boolean;
  updatedAt: string;
};

export type WindowPlacementState = {
  x: number;
  y: number;
  width: number;
  height: number;
  displayId?: string;
  updatedAt: string;
};
