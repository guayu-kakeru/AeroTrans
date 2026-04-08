export interface ReviewQueueState<T> {
  items: T[];
  index: number;
}

export function createReviewQueue<T>(items: T[], index = 0): ReviewQueueState<T> {
  if (items.length === 0) {
    return { items: [], index: -1 };
  }

  const safe = Math.min(Math.max(index, 0), items.length - 1);
  return { items: [...items], index: safe };
}

export function nextIndex(index: number, length: number): number {
  if (length <= 0) return -1;
  return (index + 1) % length;
}

export function prevIndex(index: number, length: number): number {
  if (length <= 0) return -1;
  return (index - 1 + length) % length;
}

export function removeAt<T>(state: ReviewQueueState<T>, removeIndex: number): ReviewQueueState<T> {
  if (state.items.length === 0) return { items: [], index: -1 };
  if (removeIndex < 0 || removeIndex >= state.items.length) return state;

  const nextItems = state.items.filter((_, i) => i !== removeIndex);
  if (nextItems.length === 0) return { items: [], index: -1 };

  let nextFocus = state.index;
  if (removeIndex < state.index) {
    nextFocus = state.index - 1;
  } else if (removeIndex === state.index && state.index >= nextItems.length) {
    nextFocus = nextItems.length - 1;
  }

  return { items: nextItems, index: nextFocus };
}
