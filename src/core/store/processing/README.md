# Estado compartilhado de processamento

- `client.ts`: transporte do snapshot global, polling, confirmação de controles e proteção contra respostas antigas.
- `types.ts`: contratos de workers, capacidade e retry.
- `thumbnail.ts`: consultas de outcomes em lotes para o viewport, com inscrições compartilhadas e invalidação por geração.
- Arquivos `.spec.ts`: testes junto de cada responsabilidade.

O hook `core/hooks/useProcessing.ts` possui o cliente comum usado por statusbar e duplicados. A apresentação fica em `components/features/statusbar/processing/`. A reconciliação exclusiva dos resultados de duplicados fica na própria feature, em `duplicates/state/matchingPublication.ts`.

Exemplo de importação:

```ts
import type { ProcessingSnapshot } from './types';
```

O protocolo não depende de eventos por asset para recuperar progresso ou resultados perdidos. A descrição completa está em [organização dos módulos](../../../../docs/backend/worker-module-organization.md).
