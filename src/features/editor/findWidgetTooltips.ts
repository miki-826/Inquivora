/// Monacoの検索バー（Ctrl+F）はボタンごとに title 属性を持つ。ボタンが密集しているため
/// ホバーのたびにOSのツールチップが出て、ボタンを押せなくなる。title を外して
/// aria-label へ移し、読み上げだけ残す。

function stripTooltip(element: Element): void {
  const title = element.getAttribute("title");
  if (title === null) return;
  element.removeAttribute("title");
  if (title.trim() !== "" && !element.hasAttribute("aria-label")) {
    element.setAttribute("aria-label", title);
  }
}

export function stripFindWidgetTooltips(container: Element): void {
  for (const widget of container.querySelectorAll(".find-widget")) {
    stripTooltip(widget);
    for (const element of widget.querySelectorAll("[title]")) {
      stripTooltip(element);
    }
  }
}

/// 検索バーはCtrl+Fで初めて生成され、操作のたびに作り直されるため監視し続ける。
/// 解除用の関数を返す。
export function watchFindWidgetTooltips(container: Element): () => void {
  stripFindWidgetTooltips(container);
  const observer = new MutationObserver(() => stripFindWidgetTooltips(container));
  observer.observe(container, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["title"],
  });
  return () => observer.disconnect();
}
