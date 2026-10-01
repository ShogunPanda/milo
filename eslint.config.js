import { cowtech } from '@cowtech/eslint-config'
import { fixupConfigRules } from '@eslint/compat'

// Cowtech's legacy plugins still use rule APIs removed in ESLint 10.
export default [...fixupConfigRules(cowtech)]
