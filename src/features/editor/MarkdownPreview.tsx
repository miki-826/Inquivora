import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

type MarkdownPreviewProps = {
  content: string;
};

// react-markdownは既定でHTMLをレンダリングしない（§20.3 HTML無効化）
export function MarkdownPreview({ content }: MarkdownPreviewProps) {
  return (
    <div className="markdown-preview" data-testid="markdown-preview">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{content}</ReactMarkdown>
    </div>
  );
}
