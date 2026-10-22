# Atalhos de teclado e navegação

Os atalhos abaixo usam as teclas padrão do Mundam. Os atalhos configuráveis podem ser
remapeados em **Settings → Keyboard Shortcuts**. `Meta` aparece como `⌘` no macOS e como
`Win` em Windows/Linux. A tela de configurações mostra a representação correspondente ao
sistema em uso.

`Tab` continua sendo a tecla nativa para avançar o foco. O atalho para recolher ou mostrar
os painéis é `Meta+B`, evitando interromper a navegação por teclado.

## Atalhos configuráveis por escopo

### Global

| Tecla    | Ação                                                  | Observação                                              |
| -------- | ----------------------------------------------------- | ------------------------------------------------------- |
| `Meta+K` | Focar a busca                                         | Também funciona enquanto um campo de texto está focado. |
| `Meta+,` | Abrir configurações                                   |                                                         |
| `Meta+B` | Recolher/mostrar a sidebar da biblioteca e o inspetor | Disponível na galeria.                                  |

### Viewport da galeria

| Tecla          | Ação                                                        | Observação                                  |
| -------------- | ----------------------------------------------------------- | ------------------------------------------- |
| `↑` / `↓`      | Mover o foco para cima/baixo                                | Navega pelos assets exibidos.               |
| `←` / `→`      | Mover o foco para esquerda/direita                          |                                             |
| `Home` / `End` | Focar o primeiro/último asset                               |                                             |
| `Enter`        | Abrir o asset focado                                        |                                             |
| `Space`        | Alternar seleção do asset focado                            |                                             |
| `Shift+Space`  | Adicionar/remover o asset focado sem limpar a seleção atual |                                             |
| `Meta+A`       | Selecionar todos os assets exibidos                         | Respeita a busca e os filtros ativos.       |
| `Escape`       | Limpar a seleção                                            | Só atua quando a viewport está ativa.       |
| `Delete`       | Mover os assets selecionados para a lixeira                 | Se não houver seleção, move o asset focado. |

### Visualizador de item

| Tecla                 | Ação                                    |
| --------------------- | --------------------------------------- |
| `Escape`              | Fechar o visualizador                   |
| `+` / `-`             | Aumentar/diminuir o zoom                |
| `Meta+0`              | Ajustar à tela                          |
| `Meta+1`              | Mostrar em tamanho original (100%)      |
| `H` / `R`             | Ativar ferramenta de pan/rotação        |
| `←` / `→`             | Item anterior/próximo                   |
| `Space`               | Iniciar/pausar a apresentação de slides |
| `Shift+H` / `Shift+V` | Espelhar horizontal/verticalmente       |

### Busca

| Tecla    | Ação                                                             |
| -------- | ---------------------------------------------------------------- |
| `Escape` | Limpar o texto da busca; se estiver vazio, tirar o foco do campo |

As ações “Next Result” e “Previous Result” não são atalhos separados: a navegação de assets
é feita pelas setas da viewport.

### Modais

| Tecla    | Ação                 |
| -------- | -------------------- |
| `Escape` | Fechar o modal ativo |

## Interações com mouse

| Interação                         | Ação                                                                                                       |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `Shift+clique` em um asset        | Selecionar o intervalo entre o último asset selecionado e o clicado                                        |
| `Ctrl+clique` ou `Meta+clique`    | Alternar o asset clicado sem limpar a seleção atual                                                        |
| Botão voltar do mouse             | Voltar no histórico de filtros                                                                             |
| Botão avançar do mouse            | Avançar no histórico de filtros                                                                            |
| Clique em um filtro rápido        | Ativar somente esse filtro (Untagged, Has Tags, Favorites e Trash são exclusivos); clicar de novo desativa |
| Clique em uma tag ou pasta        | Substituir a seleção por esse item; clicar no único item selecionado limpa o filtro                        |
| `Ctrl`/`Meta`+clique em tag/pasta | Adicionar/remover o item sem limpar os demais selecionados                                                 |

As interações com mouse são fixas e aparecem na tela de atalhos para consulta; não são
remapeáveis como atalhos de teclado.

## Navegação dos controles

Estes comandos são locais aos controles e não aparecem como atalhos configuráveis
(exceto os players e o localizador de duplicados, descritos abaixo):

| Controle/escopo                         | Teclas                                | Ação                                              |
| --------------------------------------- | ------------------------------------- | ------------------------------------------------- |
| Menus suspensos                         | `↑` / `↓`, `Home` / `End`             | Navegar pelos itens habilitados                   |
| Menus suspensos                         | `Enter` / `Space`                     | Ativar o item focado                              |
| Menus suspensos                         | `Escape`                              | Fechar o menu                                     |
| Menus suspensos                         | `Tab` / `Shift+Tab`                   | Navegar pelo foco nativamente                     |
| Menus de contexto                       | `↑` / `↓`, `←` / `→`                  | Navegar e abrir submenus                          |
| Menus de contexto                       | `Enter` / `Space`, `Escape`           | Ativar o item ou fechar o menu                    |
| Sugestões de tags                       | `↑` / `↓`, `Enter`, `Escape`          | Navegar, escolher/criar ou fechar sugestões       |
| Campo de tags vazio                     | `Backspace`                           | Remover a última tag                              |
| Árvore de itens                         | `Enter`, `←` / `→`                    | Selecionar, recolher ou expandir o nó focado      |
| Renomear item na árvore                 | `Enter`, `Escape`                     | Confirmar ou cancelar a edição                    |
| Select                                  | `Enter`, `Space`, `↓`                 | Abrir o seletor; `Escape` fecha a busca de opções |
| Slider                                  | `↑` / `→`, `↓` / `←`                  | Aumentar/diminuir um passo                        |
| Slider                                  | `PageUp` / `PageDown`, `Home` / `End` | Alterar por dez passos ou ir ao limite            |
| Seletor de cor                          | Setas                                 | Ajustar matiz, saturação e brilho                 |
| Controles de seleção, checkbox e switch | `Enter` / `Space`                     | Ativar o controle focado                          |
| Amostra de cor                          | `Enter` / `Space`                     | Copiar a amostra                                  |

Campos de texto, data e número mantêm a edição e a navegação nativas do teclado. Em especial,
`Tab` e `Shift+Tab` continuam movendo o foco entre os controles.

## Localizador de duplicados

Todos estes atalhos são configuráveis (escopo “Duplicate Finder”).

| Tecla     | Ação                                                                             |
| --------- | -------------------------------------------------------------------------------- |
| `↑` / `↓` | Grupo anterior/próximo                                                           |
| `←` / `→` | Candidato anterior/próximo (move o foco entre os cards)                          |
| `Space`   | Alternar a seleção do grupo ou candidato focado                                  |
| `Delete`  | Ignorar o grupo selecionado                                                      |
| `K`       | Manter os candidatos selecionados; se nenhum estiver selecionado, manter o maior |
| `S`       | Abrir o Split View (requer ao menos 2 candidatos ativos)                         |
| `R`       | Iniciar/cancelar a varredura                                                     |
| `C`       | Abrir a configuração das regras de duplicados                                    |

## Players de mídia

Configuráveis; ativos apenas enquanto o player tem o foco.

| Player | Tecla                   | Ação                                    |
| ------ | ----------------------- | --------------------------------------- |
| Vídeo  | `Space` / `K`           | Reproduzir/pausar                       |
| Vídeo  | `←` / `J`, `→` / `L`    | Voltar/avançar 5 segundos               |
| Vídeo  | `↑` / `↓`               | Aumentar/diminuir o volume              |
| Vídeo  | `F`, `M`                | Tela cheia, mudo                        |
| Áudio  | `Space`, `←` / `→`, `M` | Reproduzir/pausar, voltar/avançar, mudo |

## Navegação entre telas

Cada tela da barra de título recebe automaticamente um atalho, na ordem em que aparece
(até 9), inclusive telas adicionadas no futuro por plugins. Aparecem na tela de atalhos
na categoria Navigation e podem ser personalizados.

| Tecla                             | Ação       |
| --------------------------------- | ---------- |
| `Meta+1` (`Ctrl+1` fora do macOS) | Home       |
| `Meta+2`                          | Galeria    |
| `Meta+3`                          | Duplicados |

`Meta+1` entra em conflito com “tamanho original” no visualizador de item, que tem prioridade
enquanto aberto. `Backspace` e `Delete` são equivalentes (no macOS a tecla “delete” é `Backspace`).
