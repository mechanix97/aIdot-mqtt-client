import cv2
import os
from datetime import datetime

def crear_timelapse(fecha_str, directorio='.', salida='timelapse.avi', fps=10):
    # Validar formato de fecha
    try:
        datetime.strptime(fecha_str, '%Y%m%d')
    except ValueError:
        print("La fecha debe estar en formato YYYYMMDD.")
        return

    # Buscar archivos que empiecen con la fecha
    archivos = sorted([
        f for f in os.listdir(directorio)
        if f.startswith(fecha_str) and f.lower().endswith(('.jpg', '.jpeg', '.png'))
    ])

    if not archivos:
        print(f"No se encontraron imágenes que empiecen con {fecha_str}")
        return

    # Leer la primera imagen para obtener tamaño
    primera_img = cv2.imread(os.path.join(directorio, archivos[0]))
    alto, ancho, _ = primera_img.shape

    # Crear el writer del video
    fourcc = cv2.VideoWriter_fourcc(*'XVID')
    out = cv2.VideoWriter(salida, fourcc, fps, (ancho, alto))

    print(f"Creando timelapse con {len(archivos)} imágenes...")

    for nombre in archivos:
        path = os.path.join(directorio, nombre)
        img = cv2.imread(path)
        if img is None:
            print(f"Advertencia: no se pudo leer {path}")
            continue
        out.write(img)

    out.release()
    print(f"Timelapse guardado como {salida}")

# Ejemplo de uso
crear_timelapse('20250421', directorio='/home/lucas/home-assistant/data/cam0/', salida='output.avi', fps=15)
