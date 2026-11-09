/** The OS still draws the real traffic lights; this is only a backed drag area. */
export function Titlebar() {
  return (
    <div className="window-titlebar" data-tauri-drag-region>
      <span>MoodBuro</span>
    </div>
  );
}
