# Indexação e acompanhamento de pastas

A indexação é o processo pelo qual o Mundam conhece os arquivos guardados nas
pastas que você adiciona à biblioteca. Ela cria uma lista pesquisável dos
assets sem mover os arquivos originais para dentro do aplicativo.

## O que acontece ao adicionar uma pasta

O Mundam percorre a pasta e suas subpastas, identifica arquivos com extensões
reconhecidas e registra informações básicas, como nome, localização, formato,
tamanho e datas disponíveis no sistema de arquivos. Pastas e arquivos são
associados à localização correspondente na biblioteca.

Arquivos com formatos que não são reconhecidos não são indexados como assets.
Isso não apaga nem altera esses arquivos: eles continuam na pasta original.
Consulte [Formatos suportados](./supported-formats.md) para verificar quais
extensões o Mundam reconhece.

Na primeira varredura, os assets encontrados são adicionados ao catálogo. Nas
varreduras seguintes, o Mundam compara a localização, o tamanho e a data de
modificação para identificar itens novos, possivelmente modificados ou
inalterados. A detecção de uma modificação não garante, por si só, a
atualização de todos os dados já guardados para aquele asset.

## Acompanhamento das mudanças

As pastas adicionadas à biblioteca também são observadas pelo Mundam. O
aplicativo detecta eventos do sistema de arquivos — como criação, alteração,
renomeação, movimentação ou remoção de arquivos e pastas — e os encaminha para
atualizar a biblioteca. Eventos próximos no tempo são agrupados antes de serem
tratados, ajudando a evitar registros repetidos durante operações rápidas.

Quando uma renomeação ou movimentação pode ser reconhecida, o Mundam tenta
atualizar o caminho do asset existente em vez de tratá-lo como um arquivo
totalmente novo. Se um item desaparece da pasta, sua entrada é removida ou
atualizada no catálogo; o Mundam não usa essa detecção para apagar um arquivo
que já não está no disco.

**Importante:** embora alterações no conteúdo sejam detectadas, o código atual
não garante que metadados e miniaturas de um asset já indexado sejam
atualizados automaticamente. O acompanhamento de eventos mantém a biblioteca
informada sobre as pastas, mas não deve ser entendido como uma reextração
completa do arquivo a cada mudança.

## Trabalho em segundo plano

Depois que um asset é descoberto, tarefas separadas podem preparar seus
metadados técnicos, miniatura, paleta de cores e dados para a análise de
duplicados. Essas tarefas não precisam terminar todas ao mesmo tempo: por
exemplo, o asset pode aparecer na biblioteca antes de sua miniatura estar
pronta.

Uma varredura também pode comparar o conteúdo atual da pasta com o que já está
registrado e remover do catálogo entradas que não correspondem mais a arquivos
presentes. Isso mantém a biblioteca alinhada às pastas acompanhadas sem
confundir o catálogo do Mundam com o local onde seus arquivos originais estão
armazenados.

> **Sugestão de captura:** a área de pastas/localizações com uma pasta da
> biblioteca adicionada e, se a interface estiver disponível, o indicador de
> atividade durante uma indexação.

Veja também [metadados](./metadata.md), [miniaturas](./thumbnails.md) e
[duplicados](./operations.md#encontrar-e-revisar-duplicados).