export const createFuzzSlice = (set) => ({
  fuzzLogin: import.meta.env.VITE_DEFAULT_FUZZ_LOGIN || 'admin',
  fuzzPassword: '',
  fuzzPath: import.meta.env.VITE_DEFAULT_FUZZ_PATH || '',
  targetInput: '',
  attackType: 'RTSP_BRUTE',
  fuzzResults: [],
  sourceAnalysis: null,

  setFuzzLogin: (v) => set({ fuzzLogin: v }),
  setFuzzPassword: (v) => set({ fuzzPassword: v }),
  setFuzzPath: (v) => set({ fuzzPath: v }),
  setTargetInput: (v) => set({ targetInput: v }),
  setAttackType: (v) => set({ attackType: v }),
  setFuzzResults: (v) => set({ fuzzResults: v }),
  setSourceAnalysis: (v) => set({ sourceAnalysis: v }),
});
