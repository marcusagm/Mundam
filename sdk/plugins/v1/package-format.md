# Formato dos pacotes Mundam

## Envelope v1 · Plugins declarativos

O arquivo `.mundam-plugin` v1 carrega um manifesto e seu ícone em posições fixas. O envelope armazena os bytes originais sem compressão e sem diretório interno. Cada posição recebe exatamente um recurso. O caminho lógico do ícone vem do manifesto verificado.

| Ordem | Campo                              | Codificação                                                              |
| ----- | ---------------------------------- | ------------------------------------------------------------------------ |
| 1     | Identificador e versão do envelope | Bytes ASCII de `MUNDAM-PLUGIN`, seguidos por `00 01` em hexadecimal      |
| 2     | Tamanho do manifesto               | Inteiro de 32 bits sem sinal em big-endian, entre 1 e 65.536             |
| 3     | Tamanho do ícone                   | Inteiro de 32 bits sem sinal em big-endian, entre 1 e 262.144            |
| 4     | Manifesto                          | Bytes do manifesto v1 em UTF-8                                           |
| 5     | Ícone                              | PNG estático conforme os limites do SDK                                  |
| 6     | Chave pública do editor            | 32 bytes, codificação Ed25519 da chave pública                           |
| 7     | Assinatura                         | 64 bytes, assinatura Ed25519 sobre a concatenação exata dos campos 1 a 6 |

O tamanho máximo do arquivo é 327.799 bytes. O verificador rejeita truncamento, divergência de tamanho, versão desconhecida e bytes depois da assinatura. A assinatura opera sobre os bytes originais, sem normalizar o JSON. A separação entre o versionamento do envelope e do manifesto permite revisar o transporte em uma versão futura. [Ed25519 no RFC 8032](https://www.rfc-editor.org/rfc/rfc8032)

A política de confiança fornece um namespace exato e o SHA-256 dos 32 bytes brutos da chave pública. Esse fingerprint é diferente do hash do arquivo da chave ou de sua representação DER. O verificador compara o fingerprint esperado, confere a assinatura e valida então o manifesto e o PNG. A correspondência exige que o `publisher` validado seja igual ao namespace externo. A compatibilidade de execução é determinada no host pela versão da API e pelo alvo efetivo.

O SHA-256 do pacote completo identifica o arquivo exato que foi verificado. O futuro instalador pode vincular a aprovação a esse digest e à política de confiança vigente. O arquivo transporta uma declaração de permissões. Concessão, ativação, troca de chave e revogação pertencem à política do host.

## Envelope v2 · Worker experimental

O envelope v2 carrega um único payload Wasm associado ao campo `worker` do manifesto. A versão do manifesto continua sendo `1`. A declaração identifica o worker por um slug e uma ABI experimental. Use `preview-probe-v1` para um módulo Wasm core ou `wit-preview-check-v1` para um componente do world de conformidade `preview-check`. O manifesto deve declarar `selection.read` como obrigatória e `assetPreview.read` como obrigatória ou opcional.

```json
{
    "worker": { "id": "read-preview", "abi": "preview-probe-v1" }
}
```

| Ordem | Campo | Codificação |
| --- | --- | --- |
| 1 | Identificador e versão do envelope | Bytes ASCII de `MUNDAM-PLUGIN`, seguidos por `00 02` |
| 2 | Tamanho do manifesto | Inteiro de 32 bits sem sinal em big-endian, de 1 a 65.536 |
| 3 | Tamanho do ícone | Inteiro de 32 bits sem sinal em big-endian, de 1 a 262.144 |
| 4 | Tamanho do módulo | Inteiro de 32 bits sem sinal em big-endian, de 8 a 262.144 |
| 5 | Manifesto | JSON UTF-8 |
| 6 | Ícone | PNG estático |
| 7 | Payload | Wasm core v1 ou componente Wasm, conforme a ABI assinada |
| 8 | Chave pública do editor | 32 bytes Ed25519 |
| 9 | Assinatura | 64 bytes Ed25519 sobre os campos 1 a 8 |

O limite total permanece em 327.799 bytes e inclui todos os campos. O verificador exige que a declaração e o payload apareçam juntos. A verificação no host inclui enquadramento, assinatura, identidade do editor e cabeçalho Wasm correspondente à ABI. A validação das instruções, dos imports e do export de entrada ocorre no auxiliar.

`mundam-plugin validate` e `mundam-plugin pack` buscam `worker.wasm` ao lado de `manifest.json` quando o manifesto declara um worker. O arquivo deve existir antes de empacotar. O pacote instalado fica desabilitado até a aprovação do catálogo, e a execução experimental exige uma concessão válida para o digest exato do pacote.
