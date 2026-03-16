import { useSyncExternalStore } from 'react';

export const create = (createState) => {
  let state;
  const listeners = new Set();

  const setState = (partial, replace = false) => {
    const nextState = typeof partial === 'function' ? partial(state) : partial;
    if (nextState === state) return;
    state = replace ? nextState : Object.assign({}, state, nextState);
    listeners.forEach((listener) => listener());
  };

  const getState = () => state;

  const subscribe = (listener) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  };

  const api = { setState, getState, subscribe };
  state = createState(setState, getState, api);

  function useStore(selector = (s) => s) {
    return useSyncExternalStore(subscribe, () => selector(state), () => selector(state));
  }

  useStore.setState = setState;
  useStore.getState = getState;
  useStore.subscribe = subscribe;

  return useStore;
};
