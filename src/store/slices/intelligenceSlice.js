export const createIntelligenceSlice = (set) => ({
  intelligenceTarget: '',
  setIntelligenceTarget: (v) => set({ intelligenceTarget: v }),

  permitToken: '',
  setPermitToken: (v) => set({ permitToken: v }),

  ollamaUrl: 'http://localhost:11434',
  setOllamaUrl: (v) => set({ ollamaUrl: v }),

  ollamaModel: 'llama3',
  setOllamaModel: (v) => set({ ollamaModel: v }),

  ollamaTemperature: 0.3,
  setOllamaTemperature: (v) => set({ ollamaTemperature: v }),
});
