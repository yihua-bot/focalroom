export interface SelectionGesture { toggle?: boolean; range?: boolean }

export function selectPhotos(current: string[], anchor: string | null, id: string, order: string[], gesture: SelectionGesture = {}) {
  if (gesture.range) {
    const start = order.indexOf(anchor ?? id);
    const end = order.indexOf(id);
    const range = start >= 0 && end >= 0 ? order.slice(Math.min(start, end), Math.max(start, end) + 1) : [id];
    return gesture.toggle ? [...new Set([...current, ...range])] : range;
  }
  if (gesture.toggle) return current.includes(id) ? current.filter(item => item !== id) : [...current, id];
  return [id];
}
