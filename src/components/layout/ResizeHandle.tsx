import { useRef, useState, type PointerEvent } from "react";

type ResizeHandleProps = {
  side: "left" | "right";
  width: number;
  onResize: (width: number) => void;
};

export function ResizeHandle({ side, width, onResize }: ResizeHandleProps) {
  const [active, setActive] = useState(false);
  const startRef = useRef({ x: 0, width: 0 });

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    startRef.current = { x: event.clientX, width };
    setActive(true);
  };

  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!active) return;
    const delta = event.clientX - startRef.current.x;
    onResize(startRef.current.width + (side === "left" ? delta : -delta));
  };

  const handlePointerUp = (event: PointerEvent<HTMLDivElement>) => {
    event.currentTarget.releasePointerCapture(event.pointerId);
    setActive(false);
  };

  return (
    <div
      className={`resize-handle${active ? " resize-handle--active" : ""}`}
      role="separator"
      aria-orientation="vertical"
      aria-label="サイドバーの幅を変更"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    />
  );
}
