# Spike redroid controlado pelo Drunk

Data: 2026-10-24. Este protótipo adiciona um controlador Rust pequeno para um
Android externo. O código próprio do Drunk usa Rust, `minifb` e `image` para
exibir screenshots e encaminhar eventos básicos; Docker, ADB e a imagem Android
são componentes externos e não são inteiramente Rust.
O **NO-GO de produto permanece**: esta execução não seleciona o backend final,
resolve o inventário de licenças nem demonstra compatibilidade com Roblox.

## Ambiente observado

O laboratório existente usa Linux x86-64 com `binder_linux` carregado e Docker.
O container `drunk-redroid` iniciou Android 12 e ficou acessível por ADB em
`127.0.0.1:5555`. A imagem foi fixada em:

```text
redroid/redroid@sha256:a6c464bbedcf1dcb67dbf91f329fbb19bee5b50631f0ca6bda6ed7c41b0e64e2
```

O comando que permitiu o boot usa `--platform linux/amd64`, `--privileged`,
`--restart=no`, o volume nomeado `drunk-redroid-data:/data` e somente a publicação
`127.0.0.1:5555:5555`. Não há argumentos adicionais depois da imagem.
Mapear manualmente dispositivos Binder do host impediu o boot nesta investigação;
o comando funcional omite `--device` e deixa a imagem criar seu próprio BinderFS
interno. O host também tinha uma montagem BinderFS temporária preparada pelo
usuário; sua necessidade não foi demonstrada por essa execução.

Esses resultados descrevem o container preparado manualmente no laboratório. O
controlador Rust foi então exercitado: `status` identificou o container, `start`
confirmou a reutilização idempotente após validar sua configuração, `demo` abriu
Settings, e `stop` o encerrou. O controlador não foi usado para criar o container
inicial. O build e as consultas de ajuda/estado são verificações distintas do
ciclo completo.

## Uso da CLI

Requisitos: Docker acessível ao usuário, ADB no PATH e `binder_linux` previamente
carregado pelo administrador. A CLI não chama `sudo`, carrega módulos nem monta
sistemas de arquivos. Ela só opera no contexto Docker `default` com endpoint
local Unix; variáveis `DOCKER_HOST` e `DOCKER_CONTEXT` são recusadas para não
criar ou parar um container em outro host.

```bash
cargo build
target/debug/drunk redroid status
target/debug/drunk redroid start --allow-privileged
target/debug/drunk redroid demo
target/debug/drunk redroid view
target/debug/drunk redroid stream-probe --seconds 5
target/debug/drunk redroid stop
```

- `start` exige `--allow-privileged`, verifica `/proc/modules` e `docker info` e
  cria o container com os parâmetros fixos acima. Um container já em execução
  com a mesma imagem, modo privilegiado bridge, ADB somente em loopback, política sem
  reinicialização e volume nomeado retorna sucesso sem reiniciá-lo.
  `demo` também exige essa configuração. Se o nome existir com outra imagem ou estado, a CLI pede inspeção manual;
  não remove nem substitui o container. O sucesso de `start` informa que Docker
  iniciou o container; não garante que Android já terminou o boot.
- `status` informa o estado Docker ou `absent`. Falha de acesso ao Docker é erro,
  não ausência de container.
- `demo` conecta ADB ao endpoint fixo e solicita a atividade de Configurações
  Android por `am start -a android.settings.SETTINGS`. Pode falhar enquanto Android
  estiver inicializando; execute novamente após o boot. Antes de conectar, valida
  que o container corresponde à configuração gerenciada e rejeita configuração
  divergente. A CLI não aceita APKs,
  não instala aplicativos e não executa Roblox.
- `view` cria uma janela Linux com `minifb`, captura o framebuffer Android como
  PNG por ADB a cada 250 ms (até 4 FPS) e redimensiona a imagem para caber em
  540×900. Um clique esquerdo envia `input tap`; teclas básicas (letras, números,
  setas, Enter, espaço, Tab e Backspace) são enviadas como keyevents. Escape
  fecha a janela. O input é encaminhado como eventos discretos; manter uma
  tecla pressionada, arrastar/gestos, clipboard e áudio não estão implementados.
  A taxa de captura é baixa e cada frame inicia um processo ADB, portanto não é
  um caminho de streaming para jogos.
- `stream-probe` pede ao Android para capturar H.264 bruto por até 30 segundos
  via `adb shell -T`, conta bytes e exige NALs Annex-B de SPS, PPS e quadro.
  Separa stderr do stream e reporta diagnóstico remoto quando `screenrecord`
  falha. Informa throughput aproximado e tempo até o primeiro byte; não
  decodifica nem salva vídeo. Foi adicionado para confirmar a disponibilidade
  do transporte antes de integrar um decoder.
- `stop` valida a configuração antes de parar `drunk-redroid`. O volume de dados e
  a imagem permanecem, assim como o objeto Docker parado. Para evitar
  reutilização silenciosa, um `start` posterior encontra esse objeto e exige
  inspeção manual. A CLI não descarrega Binder nem desmonta a montagem temporária
  do host.

O modo privilegiado reduz o isolamento do container. O usuário autorizou esse uso
temporário para o laboratório; a opção deve ser fornecida em cada chamada de
`start`. A publicação ADB fica restrita ao loopback do host, mas processos locais
com acesso a esse endpoint ainda podem controlar o Android.

## Próximo incremento: stream H.264 em Rust

O caminho de screenshots valida a janela, mas não serve para jogos. O próximo
experimento deve substituir a captura por processo/frame por uma leitura contínua
de `adb shell -T screenrecord --output-format=h264 -`. A implementação AOSP do
`screenrecord` confirma que o formato H.264 bruto aceita `-` como saída em
stdout ([código AOSP](https://android.googlesource.com/platform/frameworks/av/+/297eaacfa6/cmds/screenrecord/screenrecord.cpp)).
Isso ainda precisa ser confirmado contra a imagem redroid Android 12 usada aqui.

Para preservar a implementação Rust, o processo Drunk deve ler o stream como
bytes, separar NAL units, decodificá-las em Rust e converter os frames YUV 4:2:0
para o buffer da janela `minifb`. O decoder Hibernia anuncia esse subconjunto
progressivo 8-bit 4:2:0 e licença MIT ([repositório Hibernia](https://github.com/Djuffin/hibernia)).
Há, porém, uma diferença de publicação: o crates.io oferece `hibernia` 0.1.0,
enquanto a documentação atual do repositório descreve uma API 0.2.0. Antes de
adotá-lo, comparar a API publicada com a atual e fixar uma versão ou commit
revisável; não depender implicitamente de um crate não publicado. A versão
publicada identifica-se como WIP, declara suporte Baseline e contém caminhos
`todo!()`; portanto o decoder deve ficar isolado num worker com erros/panics
convertidos em encerramento controlado do stream. O parser Annex-B entrega cada
NAL numa `Vec` apenas ao encontrar o próximo start code. O frame é planar YUV
4:2:0 e exige conversão que respeite stride/padding antes de chegar ao `minifb`.
Esses limites tornam obrigatório um ensaio com o encoder real do redroid antes
de integrar essa dependência ao comando de preview.

O probe inicial mede bytes, throughput e tempo até o primeiro byte; ainda deve
ser executado no runtime para confirmar a entrega de H.264. O experimento com
decoder deve medir FPS entregue à janela, latência aproximada de captura até
apresentação, uso de CPU e estabilidade durante vários minutos. Deve
encerrar o processo `screenrecord` ao fechar a janela ou ocorrer erro. O limite
de clipes do `screenrecord` pode exigir reinicialização supervisionada do stream.
Não integrar scrcpy nem decoder C/C++ nesse marco. Se a combinação não sustentar
uma taxa útil, registrar a medição e parar para uma decisão de arquitetura,
mantendo `view` como prévia de diagnóstico.

Este plano não demonstra áudio, aceleração gráfica, controle contínuo de teclas
ou compatibilidade Roblox. O Android screenrecord é apenas a fonte de imagem;
ele não substitui os gates de runtime, licenças e plataforma descritos na ADR.

## Limpeza do laboratório

Executado após as verificações: `drunk redroid stop`, remoção do container
`drunk-redroid`, do volume `drunk-redroid-data`, da imagem fixada, desconexão do
ADB em `127.0.0.1:5555`. `drunk redroid status` confirmou `absent` depois da
limpeza. O servidor ADB foi mantido porque também listava um endpoint adicional
`emulator-5554`; não o encerramos para evitar interromper outro dispositivo.

A montagem BinderFS em `/dev/binderfs` e o módulo `binder_linux` permanecem no
host. A tentativa de limpeza não interativa com `sudo -n` foi recusada porque
este ambiente exige autenticação interativa. Depois de confirmar que nenhum outro
processo usa BinderFS, o usuário pode executar no host:

```bash
sudo umount /dev/binderfs
sudo modprobe -r binder_linux
```

A montagem BinderFS não foi necessária para o container funcional demonstrado;
ela foi preparada durante a investigação. O módulo deve ser mantido se outro
serviço local depender dele.

## Limites deste marco

O viewer abriu uma janela nativa e recebeu frames ADB durante a execução, mas a
janela não foi inspecionada visualmente de forma independente. O encaminhamento
de toque/teclas está implementado, mas input físico não foi validado nesta rodada.
Não há integração de áudio ou controle completo das sessões. O viewer usa
capturas lentas, não um stream; isso comprova somente o caminho de janela e
decodificação de imagem. Não comprova aceleração gráfica, áudio audível ou
funcionamento de jogos. O spike não inclui espera de boot, recuperação automática,
atualização ou distribuição de imagens Android.

Redroid é um projeto externo; Drunk não é cliente oficial do Roblox. Não houve
instalação ou execução de Roblox neste spike, e não há promessa de suporte.
O gate de arquitetura/licenças e as limitações documentadas em
[runtime-evaluation.md](runtime-evaluation.md) continuam aplicáveis. Nenhuma
checagem de integridade, attestation ou restrição de plataforma é alterada.
O probe ainda não foi executado contra o container redroid Android 12; não há
evidência de saída H.264 recebida até esse ensaio ser feito.
