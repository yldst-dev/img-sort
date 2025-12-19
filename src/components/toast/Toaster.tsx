import { CSSProperties, useEffect, useState } from "react";
import { toastBus, ToastMessage } from "./toastBus";

function Toaster() {
  const [items, setItems] = useState<ToastMessage[]>([]);

  useEffect(() => {
    return toastBus.subscribe((msg) => {
      setItems((prev) => [...prev, msg]);
      setTimeout(() => {
        setItems((prev) => prev.filter((m) => m.id !== msg.id));
      }, msg.duration ?? 2200);
    });
  }, []);

  return (
    <div className="toaster">
      {items.map((m) => (
        <div
          key={m.id}
          className={`toast ${m.tone ?? "info"}`}
          style={
            {
              // Used by CSS to schedule the exit animation.
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              ["--toast-life" as any]: `${Math.max(900, m.duration ?? 2200)}ms`,
            } as CSSProperties
          }
        >
          {m.text}
        </div>
      ))}
    </div>
  );
}

export default Toaster;
