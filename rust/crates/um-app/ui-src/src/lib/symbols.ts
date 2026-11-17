// 같은 심볼을 여러 컴포넌트에서 요청해도 IPC는 한 번만 보낸다. 파일에는 저장하지 않는다.
import { invoke } from '@tauri-apps/api/core';

const cache = new Map<string, Promise<string | null>>();
export function systemSymbol(name: string, size: number, weight = 'regular'): Promise<string | null> {
  const key = JSON.stringify([name, size, weight]);
  let result = cache.get(key);
  if (!result) {
    result = invoke<string | null>('ui_symbol', { name, size, weight, color: null }).catch(() => null);
    if (cache.size >= 128) cache.clear();
    cache.set(key, result);
  }
  return result;
}
