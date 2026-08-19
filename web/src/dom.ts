// Small typed helpers for direct DOM construction. No UI framework: this keeps every component a
// plain function that builds and returns real DOM nodes.

type ElementChild = Node | string | null | undefined | false;

type ElementProps<K extends keyof HTMLElementTagNameMap> = Partial<
  Omit<HTMLElementTagNameMap[K], "children" | "style">
> & {
  class?: string;
  style?: Partial<CSSStyleDeclaration>;
  attrs?: Record<string, string>;
};

/** Creates one element with the given properties/attributes and appends the given children.
 * `attrs` sets arbitrary attributes verbatim (for things like `aria-*`/`data-*` that TypeScript's
 * DOM lib doesn't expose as direct properties); everything else in `props` is assigned as a
 * property (so event handlers like `onclick`, and IDL properties like `value`/`disabled`, work
 * exactly as they would by direct assignment). */
export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: ElementProps<K> = {},
  children: ElementChild[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  const { class: className, style, attrs, ...rest } = props;

  if (className) node.className = className;
  if (style) Object.assign(node.style, style);
  if (attrs) {
    for (const [name, value] of Object.entries(attrs)) {
      node.setAttribute(name, value);
    }
  }
  Object.assign(node, rest);

  appendChildren(node, children);
  return node;
}

export function appendChildren(node: Node, children: ElementChild[]): void {
  for (const child of children) {
    if (child === null || child === undefined || child === false) continue;
    node.appendChild(typeof child === "string" ? document.createTextNode(child) : child);
  }
}

export function clearChildren(node: Node): void {
  while (node.firstChild) node.removeChild(node.firstChild);
}

export function replaceChildren(node: Node, children: ElementChild[]): void {
  clearChildren(node);
  appendChildren(node, children);
}

export function text(value: string): Text {
  return document.createTextNode(value);
}
