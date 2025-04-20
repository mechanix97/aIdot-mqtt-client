# Usa la imagen oficial de Rust
FROM rust:latest

# Crea y usa un directorio de trabajo dentro del contenedor
WORKDIR /app

# Copia el archivo de configuración de entorno
COPY aiDot.env .

# Copia el resto del código al contenedor
COPY . .

# Instala dependencias (esto también hace que cargo compile todo)
RUN cargo build --release

# Carga variables de entorno y ejecuta el programa
# ENTRYPOINT no puede hacer esto directamente, así que usamos un script
RUN echo '#!/bin/sh\n\
set -a\n\
. ./aidot.env\n\
set +a\n\
cargo run --release' > start.sh && chmod +x start.sh

CMD ["./start.sh"]
