module.exports = {
  root: true,
  env: { browser: true, es2022: true },
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
  ],
  parser: '@typescript-eslint/parser',
  parserOptions: { ecmaVersion: 'latest', sourceType: 'module' },
  plugins: ['@typescript-eslint', 'react-hooks'],
  ignorePatterns: ['dist', 'src-tauri', 'node_modules', '*.cjs', 'vite.config.ts'],
  rules: {
    ...require('eslint-plugin-react-hooks').configs.recommended.rules,
    '@typescript-eslint/no-unused-vars': [
      'error',
      { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
    ],
    // `any` is banned; the IPC boundary uses `unknown` plus narrowing.
    '@typescript-eslint/no-explicit-any': 'error',
    'no-console': ['warn', { allow: ['warn', 'error'] }],
    eqeqeq: ['error', 'always', { null: 'ignore' }],
  },
  overrides: [
    {
      files: ['**/*.test.ts', '**/*.test.tsx', 'src/test/**'],
      env: { node: true },
      rules: { '@typescript-eslint/no-explicit-any': 'off' },
    },
  ],
};
