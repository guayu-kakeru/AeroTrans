export function resolveHydratedValue<T>(
  current: T,
  loaded: T,
  dirtyDuringBootstrap: boolean,
): T {
  return dirtyDuringBootstrap ? current : loaded;
}

export const INTERACTIVE_BOOTSTRAP_STATUS = "可用，正在同步配置...";
