# Mundam - Frontend Overview

## Folder structure
```
Mundam/src
├── assets/
├── components/
│   ├── features/
│   ├── layout/
│   └── ui/
├── config/
├── core/
│   ├── dnd/
│   ├── hooks/
│   ├── input/
│   ├── services/
│   ├── store/
│   │   └── processing/
│   ├── tauri/
│   ├── tests/
│   ├── types/
│   ├── utils/
│   └── viewport/
├── layouts/
├── lib/
├── styles/
│   └── themes/
├── types/
├── utils/
├── App.tsx
├── mountApplication.tsx
├── splash.tsx
└── index.tsx
```

## Core

### Drag and drop system 

### Hooks

### Input

### Services

### Store

O estado compartilhado dos workers fica em [core/store/processing](../../src/core/store/processing/README.md), com cliente, contratos, read model de thumbnails e testes. O hook `useProcessing` compartilha o cliente entre statusbar, configurações e duplicados.

A reconciliação de publicações exclusiva de duplicados fica em `components/features/duplicates/state/`.

### System

### Tauri

### Tests

### Types

### Utils

### Viewport

## Components

### Features

A subseção de processamento da statusbar fica em `components/features/statusbar/processing/`, com componentes, apresentação, CSS e testes juntos. Configurações de performance continuam na feature `settings`.

A auditoria de módulos, limpeza e pendências do plano está em [Organização dos módulos](../backend/workers/2026-11-04-19-02-worker-module-organization.md).

### Layouts

### UI


