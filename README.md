# getClean

Um organizador pequeno de Downloads, escrito em Rust e sem dependências externas. Ele examina apenas os arquivos na raiz de Downloads a cada cinco segundos e os move para as pastas do usuário. Subpastas e arquivos com extensões desconhecidas ficam intactos.

## Destinos

| Extensões | Destino |
| --- | --- |
| jpg, jpeg, png, gif, webp, svg, bmp, ico | `Pictures/jpeg`, `Pictures/png`, etc. |
| mp4, mkv, avi, mov, webm | `Videos/mp4`, `Videos/mkv`, etc. |
| mp3, wav, flac, ogg | `Music/mp3`, `Music/wav`, etc. |
| pdf, docx, xlsx, pptx, txt | `Documents/pdf`, `Documents/word`, `Documents/excel`, `Documents/powerpoint`, `Documents/txt` |
| zip, rar, 7z, tar, gz | `Documents/Archives` |
| exe, msi, dmg, iso | `Documents/Programs` |

Por exemplo, `Downloads/relatorio.PDF` vai para `Documents/pdf/relatorio.PDF`. As extensões não diferenciam maiúsculas de minúsculas. Nomes repetidos recebem ` (1)`, ` (2)` etc., sem substituir arquivos existentes. No Windows, as pastas do sistema são localizadas pela API do próprio Windows, incluindo quando foram redirecionadas para OneDrive.

## Uso

Instale o [Rust](https://rustup.rs/) e execute:

```sh
cargo run --release
```

O programa organiza arquivos antigos ao iniciar e continua rodando. Arquivos recentes precisam permanecer sem alterações durante dez segundos. Arquivos temporários de download, como `.part` e `.crdownload`, são ignorados. Erros de leitura ou movimentação são mostrados e uma nova tentativa é feita na próxima varredura.

```sh
cargo run --release -- --downloads "/caminho/para/Downloads"
cargo run --release -- --once
cargo run --release -- --dry-run
cargo run --release -- --history
cargo run --release -- --undo
cargo run --release -- --interval 15 --settle 20
```

`--once` faz uma varredura e sai; arquivos recentes podem ainda não estar prontos. `--dry-run` mostra o que seria movido sem mover e também faz uma só varredura. `--history` mostra os últimos 20 movimentos; `--undo` devolve o último movimento ainda não desfeito à pasta de origem. Se o nome original já estiver ocupado, o arquivo devolvido recebe um sufixo. Um arquivo devolvido é ignorado pelo organizador enquanto continuar sem alterações, para evitar que seja movido novamente.

`GETCLEAN_DOWNLOADS` define outra pasta de origem; `--downloads` tem prioridade. O histórico fica em `%LOCALAPPDATA%\getClean\history.log` no Windows ou em `$XDG_STATE_HOME/getclean/history.log` (por padrão `~/.local/state/getclean/history.log`) no Linux. `--state-dir` altera esse local.

## Execução contínua

### Linux

Rode `sh scripts/install-linux.sh`. O script compila em modo release, instala em `~/.local/bin` e ativa um serviço de usuário do systemd com reinício automático. Veja o estado com `systemctl --user status getclean`. Para continuar rodando depois de sair da sessão, um administrador pode ativar `sudo loginctl enable-linger "$USER"`. Para remover, execute `systemctl --user disable --now getclean`.

### Windows

Rode `cargo build --release` e depois `powershell -ExecutionPolicy Bypass -File scripts/install-windows.ps1`. Se o build reclamar da ausência de `link.exe`, use `rustup toolchain install stable-gnu` e `cargo +stable-gnu build --release`. Isso cria uma Tarefa Agendada para o usuário atual, inicia imediatamente e volta a iniciar ao entrar na sessão. A tarefa não tem limite de duração e tenta reiniciar se o programa falhar. Veja com `Get-ScheduledTask -TaskName getClean` e remova com `Unregister-ScheduledTask -TaskName getClean -Confirm:$false`.

Para usar outra pasta de origem, passe `-Downloads 'D:\Downloads'` ao instalador. O instalador copia o executável para `%LOCALAPPDATA%\getClean\getclean.exe`, de modo que a tarefa continue funcionando mesmo se o repositório for movido.

Se a versão antiga em Go estiver instalada como serviço, pare e remova esse serviço antes de ativar a versão em Rust. Assim, os dois organizadores não disputarão os mesmos arquivos.

## Desenvolvimento

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo build --release
```

O workflow do GitHub Actions executa compilação e testes no Windows e no Linux.

## Licença

MIT. Consulte [LICENSE](LICENSE).
