import { create } from 'zustand';
import { createSpiderSlice } from './slices/spiderSlice';
import { createFtpSlice } from './slices/ftpSlice';
import { createFuzzSlice } from './slices/fuzzSlice';
import { createMassAuditSlice } from './slices/massAuditSlice';

export { SPIDER_MODULES_CONFIG } from './slices/spiderSlice';

export const useAppStore = create((...a) => ({
  ...createSpiderSlice(...a),
  ...createFtpSlice(...a),
  ...createFuzzSlice(...a),
  ...createMassAuditSlice(...a),

  hubCookie: '',
  setHubCookie: (v) => a[0]({ hubCookie: v }),
}));
