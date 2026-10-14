# Onde você vê os assets

O Mundam apresenta os assets em diferentes áreas. Cada uma serve a um
propósito: navegar pela coleção, examinar um arquivo ou consultar seus dados.

## Viewport

A [Viewport](./views.md#viewport) é a área principal de navegação pelos assets.
Você pode alternar entre os modos de mosaico vertical, mosaico horizontal,
grade e lista e ajustar o tamanho das miniaturas.

Nas opções de visualização, também é possível escolher se os dados aparecem
sobre a miniatura ao passar o cursor ou abaixo dela e selecionar quais campos
ficam visíveis. Os campos disponíveis incluem nome, extensão, dimensões,
tamanho, avaliação, datas de criação/modificação/adicionamento e tags.

> **Sugestão de captura:** a Viewport em modo de grade ou lista, com as opções
> de layout e os campos de informação abertos.

## Item View

O [Item View](./views.md#item-view) abre o asset selecionado em uma área maior.
O conteúdo depende do tipo de arquivo:

- imagens e alguns formatos de projeto podem abrir como pré-visualização;
- áudio e vídeo usam controles próprios de reprodução;
- modelos 3D com suporte podem ser apresentados em um visualizador;
- fontes compatíveis oferecem ferramentas de visualização tipográfica.

As ferramentas disponíveis variam por tipo. Na visualização de imagens, por
exemplo, há controles para zoom e ajuste à tela, além de ferramentas de
navegação e apresentação. Não é um editor de arquivos: mudanças feitas na
visualização não substituem um aplicativo especializado para editar o conteúdo
original.

> **Sugestão de captura:** uma imagem aberta no Item View com a barra de
> ferramentas visível; outra possibilidade é mostrar um modelo 3D ou uma fonte.

## Inspector

O [Inspector](./views.md#inspector) mostra detalhes do asset selecionado e,
para alguns campos, permite organizar o asset sem sair da biblioteca.

Em geral, a seção **General Info** apresenta nome, tipo, tamanho, datas,
favorito, avaliação e anotações. As tags também podem ser consultadas e
editadas. As demais seções mudam conforme o tipo de arquivo:

- **Imagem**: dimensões, megapixels, identificador, dados avançados disponíveis
  (incluindo EXIF quando extraído) e [paleta de cores](./colors.md).
- **Áudio**: controles de reprodução e informações gerais do asset.
- **Vídeo**: pré-visualização com controles de reprodução e informações gerais.
- **Fonte**: amostra visual, formato e extensão.
- **Modelo 3D**: miniatura, formato e informações do modelo disponíveis no
  Inspector.
- **Vários assets**: quantidade selecionada e organização por tags em lote.

Campos específicos podem estar vazios ou não aparecer quando o formato não
fornece o dado correspondente.

> **Sugestão de captura:** o Inspector de imagem é útil para mostrar como
> miniatura, metadados, tags, dados avançados e cores ficam reunidos. Para
> explicar a seleção múltipla, mostre o Inspector com vários assets
> selecionados.
