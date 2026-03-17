export const createMassAuditSlice = (set) => ({
  massAuditIps: '',
  massAuditLogin: 'admin',
  massAuditPass: '',
  massAuditResults: [],
  massAuditing: false,

  setMassAuditIps: (v) => set({ massAuditIps: v }),
  setMassAuditLogin: (v) => set({ massAuditLogin: v }),
  setMassAuditPass: (v) => set({ massAuditPass: v }),
  setMassAuditResults: (v) => set({ massAuditResults: v }),
  setMassAuditing: (v) => set({ massAuditing: v }),
});
