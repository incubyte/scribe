import js from "@eslint/js";
import jsxA11y from "eslint-plugin-jsx-a11y";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "node_modules", "src-tauri"] },
  js.configs.recommended,
  tseslint.configs.recommended,
  jsxA11y.flatConfigs.recommended,
  {
    rules: {
      "max-depth": ["error", 3],
      complexity: ["error", 5],
      "no-param-reassign": "error",
      "no-console": "warn",
      eqeqeq: "error",
      curly: "error",
      "no-var": "error",
      "prefer-const": "error",
    },
  },
);
