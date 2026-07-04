type PanePlaceholderProps = {
  title: string;
  description?: string;
};

export function PanePlaceholder({ title, description }: PanePlaceholderProps) {
  return (
    <div className="placeholder">
      <div className="placeholder__title">{title}</div>
      {description && <div>{description}</div>}
    </div>
  );
}
