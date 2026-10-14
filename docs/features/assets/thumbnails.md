# Miniaturas e pré-visualizações

Miniaturas ajudam você a reconhecer arquivos rapidamente na biblioteca.
Pré-visualizações permitem examinar alguns tipos de asset em uma visualização
maior. Elas são representações preparadas pelo Mundam; não substituem nem
alteram o arquivo original.

## Miniaturas na biblioteca

Quando há um extrator disponível, o Mundam gera uma miniatura para a Viewport.
O resultado é normalizado para WebP e salvo no cache do aplicativo para poder
ser reutilizado. A geração acontece em segundo plano, com um número limitado
de tarefas simultâneas. Assets que ainda não têm miniatura podem aparecer
temporariamente com um espaço reservado ou ícone.

Alguns formatos têm uma imagem de pré-visualização incorporada; outros
precisam ser interpretados ou convertidos. Em tipos sem miniatura visual
específica, a apresentação pode usar um ícone genérico. A disponibilidade
depende do formato e de seus extratores.

## Pré-visualização ao abrir um asset

Ao abrir um asset, o [Item View](./views.md#item-view) apresenta a melhor
visualização que o formato e o aplicativo permitem. Para certos tipos de
imagem, a pré-visualização é preparada somente quando necessária e pode ser
guardada em cache separadamente da miniatura. Modelos 3D com suporte também
podem receber um recurso de visualização próprio.

Áudio e vídeo usam controles de reprodução; alguns formatos de vídeo precisam
ser preparados para streaming. O áudio pode exibir uma forma de onda extraída
separadamente pelo FFmpeg. Arquivos MIDI passam por uma etapa de síntese de
áudio antes dessa análise e precisam de uma SoundFont disponível. A forma de
onda não é a miniatura nem determina como o áudio é reproduzido. A extração
pode falhar em alguns arquivos; a coluna **Waveform extraction** na tabela de
[formatos suportados](./supported-formats.md#audio-formats) indica os testes
registrados por extensão.

Os formatos não oferecem todos os mesmos recursos. Consulte
[Formatos suportados](./supported-formats.md) para conferir estratégias,
dependências e limitações conhecidas.

> **Sugestão de captura:** a mesma imagem como miniatura na Viewport e aberta em
> tamanho maior no Item View. Também pode ser útil mostrar um asset de áudio ou
> vídeo com seus controles de reprodução.

Saiba como a miniatura também pode alimentar a [extração de cores](./colors.md).