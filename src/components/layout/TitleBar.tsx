interface TitleBarProps {
  title: string;
}

function TitleBar({ title }: TitleBarProps) {
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar-title" data-tauri-drag-region>
        {title}
      </div>
    </header>
  );
}

export default TitleBar;
