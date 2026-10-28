import type { CanvasPhoto } from '../../domain/canvas';

/** 保持插入区域在当前视口附近，必要时向下避开已有内容。 */
export function insertionY(nodes: CanvasPhoto[], x: number, y: number, width: number, height: number) {
  const obstacles = nodes.filter(node => node.visible !== false && node.nodeType !== 'arrow' && node.x < x + width && node.x + node.width > x)
    .sort((a, b) => a.y - b.y);
  for (const node of obstacles) {
    if (node.y < y + height && node.y + node.height > y) y = node.y + node.height + 24;
  }
  return y;
}
