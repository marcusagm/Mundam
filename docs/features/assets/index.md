# Trabalhando com assets

No Mundam, um *asset* é um arquivo que faz parte de uma das pastas adicionadas
à biblioteca. O aplicativo registra onde ele está, identifica seu formato e,
quando há suporte, prepara informações e imagens de pré-visualização para
facilitar a organização e a busca. Os arquivos originais continuam nas pastas
escolhidas por você; tags, avaliações e anotações são informações organizadas
pelo Mundam.

## Como o Mundam trabalha com seus arquivos

1. **Você adiciona uma pasta à biblioteca.** O Mundam percorre essa pasta e
   suas subpastas e registra os arquivos com formatos reconhecidos.
2. **As informações são preparadas em segundo plano.** Conforme o formato,
   podem ser extraídos metadados, uma miniatura, dados de cores e uma impressão
   digital para encontrar duplicados.
3. **Você explora os resultados.** A [Viewport](./views.md#viewport) organiza os
   assets em lista, grade ou mosaico; o [Item View](./views.md#item-view) abre
   uma visualização maior; e o [Inspector](./views.md#inspector) mostra os
   detalhes disponíveis.
4. **O Mundam acompanha as pastas adicionadas.** Quando arquivos são criados,
   alterados, movidos ou removidos, o índice pode ser atualizado para refletir
   essas mudanças.

Nem todo formato oferece os mesmos recursos. A disponibilidade de miniaturas,
pré-visualizações, metadados e reprodução depende do formato e de seus
extratores. Consulte [Formatos suportados](./supported-formats.md) para ver o
suporte detalhado.

## Guia

- [Indexação e acompanhamento de pastas](./indexer.md) — como a biblioteca
  encontra e mantém os assets atualizados.
- [Miniaturas e pré-visualizações](./thumbnails.md) — o que aparece nas listas
  e como os arquivos são abertos.
- [Metadados e organização](./metadata.md) — dados do arquivo, tags e
  informações pessoais.
- [Cores, paletas e harmonia](./colors.md) — como o Mundam identifica cores e
  permite procurá-las.
- [Ações e duplicados](./operations.md) — o que você pode fazer com um ou mais
  assets.
- [Onde cada informação aparece](./views.md) — explicação da Viewport, do Item
  View e do Inspector.
- [Formatos suportados](./supported-formats.md) — lista técnica detalhada por
  formato.

> **Sugestão de captura:** uma imagem da biblioteca aberta, mostrando a
> navegação de pastas, a Viewport e o Inspector com um asset selecionado.
