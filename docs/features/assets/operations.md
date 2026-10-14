# Ações e duplicados

O Mundam oferece ações para organizar e abrir os assets da biblioteca. As
opções disponíveis dependem da seleção e de o asset estar na biblioteca ou na
lixeira.

## Ações sobre arquivos

No menu de contexto de um asset, você pode:

- abrir o arquivo no aplicativo associado do sistema;
- revelar o arquivo na pasta do sistema;
- copiar o arquivo ou copiar seu caminho como texto;
- renomear um único arquivo — a extensão original é preservada;
- mover o asset para a lixeira do Mundam.

Com vários assets selecionados, você pode abrir os arquivos, copiar os arquivos
ou seus caminhos e mover a seleção para a lixeira. Para itens que já estão na
lixeira, o menu oferece a restauração. A ação **Undo** também permite desfazer
uma movimentação recente para a lixeira.

Mover para a lixeira do Mundam é diferente de apagar definitivamente: o item
pode ser restaurado. Esvaziar a lixeira é a ação destinada à remoção
permanente; confira com atenção antes de usá-la.

## Organização em lote

O [Inspector](./views.md#inspector) aceita uma seleção múltipla. Nesse modo,
você pode revisar e editar tags compartilhadas entre os assets. Adicionar ou
remover uma tag nesse contexto aplica a mudança aos itens selecionados.

Para um único asset, o Inspector também permite alternar o favorito, escolher
uma avaliação de zero a cinco estrelas e editar anotações. Consulte
[Metadados e organização](./metadata.md) para saber quais informações são
extraídas e quais são adicionadas por você.

## Encontrar e revisar duplicados

O Mundam pode criar impressões digitais dos arquivos para procurar cópias com
conteúdo idêntico. Para imagens, também compara características visuais para
encontrar versões parecidas, como imagens recomprimidas ou recortadas. Uma
semelhança visual é uma indicação para revisar, não uma garantia de que dois
arquivos sejam intercambiáveis.

Você pode iniciar uma análise de duplicados e revisar os grupos encontrados.
Na comparação, pode escolher quais itens manter, ignorar um grupo ou usar as
ações sugeridas para decidir. Antes de concluir uma resolução, o Mundam permite
revisar uma mesclagem de metadados campo a campo. Os itens descartados por uma
resolução são enviados à lixeira, não apagados permanentemente; a resolução
pode ser desfeita para reabrir o grupo e restaurar os itens.

As regras de identificação de duplicados podem ser configuradas. A detecção
automática de cópias idênticas ao adicionar arquivos também pode ser ligada ou
desligada nas configurações.

## Operações e segurança

O Mundam registra internamente determinadas alterações feitas no catálogo,
incluindo dados necessários para acompanhar operações que envolvem o banco de
dados e os arquivos. Isso ajuda o aplicativo a concluir ou recuperar algumas
operações interrompidas.

Esse registro interno não é uma tela de histórico de atividades para o
usuário. As alterações detectadas nas pastas adicionadas são tratadas pelo
[indexador](./indexer.md); ações de gerenciamento e revisão são descritas
acima.

> **Sugestão de captura:** o menu de contexto de um asset e, em outra imagem,
> a tela de comparação de duplicados com a revisão de metadados.

Consulte também [Indexação e acompanhamento de pastas](./indexer.md) e os
[formatos suportados](./supported-formats.md).