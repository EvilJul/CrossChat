import { useEffect } from "react";
import { register, isRegistered } from "@tauri-apps/plugin-global-shortcut";
import { getCurrentWindow } from "@tauri-apps/api/window";
import CanvasView from "./components/canvas/CanvasView";
import WelcomeDialog from "./components/WelcomeDialog";

function App() {
  useEffect(() => {
    const win = getCurrentWindow();
    let unlistenFocus: (() => void) | null = null;

    const setupShortcut = async () => {
      try {
        const shortcut = "Alt+Space";
        const registered = await isRegistered(shortcut);
        if (!registered) {
          await register(shortcut, async (event) => {
            if (event.state === "Pressed") {
              const visible = await win.isVisible();
              if (visible) {
                await win.hide();
              } else {
                await win.show();
                await win.setFocus();
              }
            }
          });
        }
      } catch (err) {
        console.warn("Global shortcut registration failed:", err);
      }
    };

    const setupFocusBlur = async () => {
      try {
        const unlisten = await win.onFocusChanged(({ payload: focused }) => {
          if (!focused && !import.meta.env.DEV) {
            win.hide().catch(console.error);
          }
        });
        unlistenFocus = unlisten;
      } catch (err) {
        console.warn("Focus listener failed:", err);
      }
    };

    setupShortcut();
    setupFocusBlur();

    return () => {
      if (unlistenFocus) unlistenFocus();
    };
  }, []);

  return (
    <>
      <CanvasView />
      <WelcomeDialog />
    </>
  );
}

export default App;
