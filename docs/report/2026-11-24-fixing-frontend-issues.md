# Correções de foco, barra de status e movimento da interface

Implementação de 24 de novembro de 2026. O trabalho abrange o frontend Solid da biblioteca e as janelas auxiliares de inicialização e encerramento.

## Foco e edição segura

Na grade e no mosaico, o identificador do asset em foco também define a célula alcançável por Tab. O componente compartilhado `AssetItemContainer` aplicava foco nativo sempre que montava com esse identificador. A virtualização pode remontar uma célula durante uma atualização de resultados, mesmo que o usuário esteja digitando fora da galeria.

A sincronização agora verifica `document.activeElement`. Só transfere o foco se ele pertence à própria grade ou a uma célula daquela mesma grade. Assim, uma mudança de thumbnail, seleção ou resultado pode atualizar a aparência do cartão enquanto o cursor de texto permanece na busca, no inspector ou na sidebar. A tabela usa seu próprio caminho de foco associado ao clique na linha.

O comando de lixeira da grade e do mosaico também verifica se o contêiner de rolagem contém o foco antes de agir. Isso cobre inclusive o caso em que a remoção de uma célula deixa o foco no documento. A proteção de campos editáveis do dispatcher continua a atender aos atalhos globais.

## Composição das telas

A galeria era a única consumidora do `AppShell` e a barra de status era a única consumidora de seu contexto. `GalleryView.tsx` agora compõe diretamente o cabeçalho, a biblioteca, o viewport, o inspector, os divisores e o rodapé. O CSS correspondente fica em `src/views/gallery-view.css`. As chaves `app-shell-layout` e `app-shell-states` preservam os tamanhos e a visibilidade já salvos. O comando de alternância dos painéis acompanha o ciclo de vida da galeria.

`src/components/layout/GlobalStatusbar.tsx` concentra a mensagem e o painel de atividade do sistema. Cada tela fornece seus elementos locais pelas propriedades opcionais `context` e `actions`. Os componentes de layout compartilhados continuam em `src/components/layout`. O modal de configurações também está disponível antes da escolha da pasta da biblioteca.

| Tela | Conteúdo específico do rodapé |
| --- | --- |
| Galeria | Contagens de assets carregados, filtrados e selecionados, controles da sidebar e do inspector |
| Duplicados | Quantidade de grupos visíveis segundo os filtros de tipo e de estado, calculada sobre a mesma coleção que alimenta a lista |
| Início e boas-vindas | Apenas informações e ações comuns |

## Preferência de movimento

Em **Settings → Performance → Interface motion**, **Follow system** acompanha a configuração de acessibilidade do sistema e é o padrão para instalações existentes. **Reduce motion** reduz animações, transições e rolagem suave. **Full motion** permite os efeitos normais mesmo com redução ativada no sistema.

A escolha é local ao dispositivo. `src/core/store/motionPreference.ts` lê a chave `mundam-motion-preference` antes da renderização e publica o resultado efetivo em `data-reduced-motion` no elemento HTML. As três entradas de janela inicializam esse comportamento. Mudanças na consulta de mídia ou no armazenamento de outra janela recalculam o resultado. Valores ausentes ou desconhecidos usam o sistema.

O CSS global reduz durações e atrasos, inclusive em pseudo-elementos. A tela inicial também suprime transformações decorativas e o indicador da abertura e do encerramento assume uma apresentação estática. A navegação por teclado consulta a mesma preferência efetiva ao rolar até um asset.

## Cobertura e roteiro de verificação

O teste de regressão de `AssetItemContainer` monta e remonta um cartão virtualmente focado enquanto um campo externo está ativo. Em seguida, transfere o foco para a grade e verifica a navegação até a célula. O teste de `motionPreference` cobre mudanças no sistema, as duas escolhas explícitas e sincronização entre janelas.

Para homologação na aplicação desktop:

1. Com um asset selecionado e miniaturas sendo geradas, edite uma busca, uma palavra-chave e uma tag. Verifique a posição do cursor e use Delete dentro do texto. Confirme que a seleção e o arquivo continuam disponíveis.
2. Nas visualizações de grade e mosaico, entre com Tab e navegue com as setas. Confira a seleção e a rolagem. Para validar o comando de lixeira, use um arquivo descartável.
3. Durante um processamento, alterne entre início, galeria e duplicados. Abra os detalhes de atividade em cada tela. Em duplicados, altere os filtros e compare o contador com a lista.
4. Recolha os painéis da galeria, navegue para outra tela e volte. Reabra o aplicativo e confira a restauração dos painéis e das proporções.
5. Alterne as três opções de movimento. Confira os efeitos da tela inicial, a rolagem da grade e o indicador ao reabrir o aplicativo. Em Follow system, altere também a preferência do sistema com o aplicativo aberto.

## Resultados

- TypeScript, ESLint e build de produção do Vite concluídos com sucesso.
- Os dois testes de regressão adicionados passaram em execuções direcionadas.
- Na execução geral iniciada antes da inclusão desses testes, 251 de 252 testes passaram. O caso de recuperação de pastas excedeu o limite de 5 segundos. Na repetição com um worker e limite de 15 segundos, passou em 5,23 segundos. A configuração da suíte foi preservada.
- A verificação de foco automatizada utilizou o DOM do ambiente de testes. Na homologação manual posterior, o usuário confirmou a preservação do foco na busca, nas palavras-chave e nas tags durante atualizações, o escopo de Delete, a contagem de grupos listados, a persistência dos painéis e a redução de movimento após reiniciar.

## Segunda rodada de ajustes

**Duplicados.** O rodapé usa a coleção completa para o total de grupos e para os totais por estado (resolvidos e ignorados). “Shown” conta os grupos que atendem aos filtros da lista. “Assets selected” conta os candidatos marcados no grupo em comparação. Uma seleção com dois candidatos exibe dois assets selecionados, mesmo quando nenhum asset está selecionado na galeria. Ocultar o grupo pelos filtros ou trocar de grupo encerra aquela seleção.

**Boas-vindas.** Enquanto não há pasta monitorada, a barra de título preserva a área de arraste e os controles da janela. Os botões de navegação e seus atalhos são disponibilizados quando a biblioteca está configurada.

**Clipboard no Windows.** O comando Tauri anteriormente retornava sucesso no Windows sem escrever no clipboard. Agora utiliza o suporte a listas de arquivos de arboard para publicar caminhos no formato nativo CF_HDROP. O Explorer pode consumir a lista em uma operação de colar. O comando aceita seleção múltipla, verifica se os arquivos existem e comunica falhas pelo tratamento de erros do IPC. “Copy Path” continua sendo a ação de texto. O macOS usa a implementação AppKit existente.

**Inspector.** Em General Info, o nome completo é texto selecionável com quebra de linha, inclusive quando não há espaços. O lápis abre o mesmo diálogo de renomeação utilizado pelo menu de contexto. Os dois capturam o identificador do asset ao abrir o diálogo e sugerem o nome sem a última extensão. Arquivos ocultos com um único ponto inicial conservam o nome completo. O backend da biblioteca continua responsável pela extensão, pela validação de colisões e pela alteração física. A edição fica disponível para um único asset ativo com caminho.

### Homologação específica da segunda rodada

1. Em uma biblioteca com grupos abertos, resolvidos e ignorados, alterne os filtros de tipo e estado. Compare “Shown” com a lista e confira se os totais globais permanecem estáveis. Marque dois candidatos e troque de grupo para verificar a reinicialização da seleção.
2. Abra o aplicativo sem biblioteca configurada. Verifique a ausência dos seletores de tela e tente os atalhos de navegação. Escolha a pasta e verifique o retorno dos controles.
3. No Windows, copie um arquivo e depois uma seleção com espaços e acentos nos nomes. Cole em uma pasta de teste no Explorer e confira a quantidade e os nomes. Compare com “Copy Paths” colado em um editor de texto.
4. No inspector estreito, selecione um arquivo de nome longo. Renomeie pelo lápis, confira o nome e a extensão no disco e na galeria. Tente um nome em conflito e cancele para conferir a recuperação.

### Validação da segunda rodada

ESLint e TypeScript passaram. O backend compilou na checagem do alvo Windows com `cargo check --locked --offline --lib`. Passaram os seis testes direcionados de `FolderActionsDialog` e `DiscoveryMetadata`, incluindo a recuperação após erro de renomeação.

### Homologação concluída

Na segunda rodada, o usuário confirmou os quatro resultados no aplicativo.

- Os contadores de grupos duplicados e de assets selecionados apresentaram os valores esperados.
- A tela de boas-vindas ocultou os botões de acesso a Home, Gallery e Duplicates.
- A cópia no Windows funcionou com um ou vários arquivos, inclusive com caracteres especiais nos nomes.
- A renomeação de arquivos funcionou corretamente.

Com as duas rodadas homologadas, o trabalho foi encerrado. O [inventário de dependências](../backend/dependences.md) registra a integração de clipboard do Windows.
