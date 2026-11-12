import java.lang.reflect.Field;
import zombie.iso.*;

// Inside the booted original (ServerHarness): a meta grid built by IsoMetaGrid.Create, for a cell range.
// Args: <scratch dir> <game map (informational, the server default is used)> x1 y1 x2 y2
public class MetaGridProbe {
    static long h;
    static void mix(int v) { for (int i = 0; i < 4; i++) { h ^= (v >>> (8 * i)) & 255; h *= 0x100000001b3L; } }
    static void mixLong(long v) { mix((int) v); mix((int) (v >> 32)); }

    public static void main(String[] a) throws Exception {
        int x1 = Integer.parseInt(a[2]), y1 = Integer.parseInt(a[3]), x2 = Integer.parseInt(a[4]), y2 = Integer.parseInt(a[5]);
        ServerHarness.run(a[0], out -> {
            // A second grid built by the original Create(): the world's own grid has basements merged in by now
            // (zombie.basements.Basements, not ported yet), which renumbers rooms and adds buildings.
            IsoMetaGrid g = new IsoMetaGrid();
            g.Create();
            StringBuilder dirs = new StringBuilder();
            for (MapFiles m : IsoLot.MapFiles) { if (dirs.length() > 0) dirs.append(';'); dirs.append(m.mapDirectoryName); }
            out.println("G " + g.minX + " " + g.minY + " " + g.maxX + " " + g.maxY + " dirs=" + dirs);
            Field bf = IsoMetaGrid.class.getDeclaredField("buildings"); bf.setAccessible(true);
            java.util.ArrayList<BuildingDef> buildings = (java.util.ArrayList<BuildingDef>) bf.get(g);
            h = 0xcbf29ce484222325L;
            for (BuildingDef b : buildings) mixLong(b.id);
            out.println("B " + buildings.size() + " " + Long.toHexString(h));
            Field rf = IsoMetaChunk.class.getDeclaredField("rooms"); rf.setAccessible(true);
            Field rs = IsoMetaChunk.class.getDeclaredField("roomsSize"); rs.setAccessible(true);
            for (int cx = x1; cx <= x2; cx++) for (int cy = y1; cy <= y2; cy++) {
                LotHeader info = IsoLot.getHeader(cx, cy);
                if (info == null) continue;
                StringBuilder sb = new StringBuilder("A " + cx + " " + cy + " ");
                for (boolean b : info.adjacentCells) sb.append(b ? '1' : '0');
                sb.append('\n');
                IsoMetaCell cell = g.getCellData(cx, cy);
                for (int ly = 0; ly < 32; ly++) for (int lx = 0; lx < 32; lx++) {
                    if (!cell.hasChunk(lx, ly)) continue;
                    IsoMetaChunk ch = cell.getChunk(lx, ly);
                    int n = rs.getInt(ch);
                    if (n == 0) continue;
                    RoomDef[] rooms = (RoomDef[]) rf.get(ch);
                    sb.append("K ").append(cx).append(' ').append(cy).append(' ').append(lx).append(' ').append(ly);
                    for (int i = 0; i < n; i++) sb.append(' ').append(rooms[i].id);
                    sb.append('\n');
                }
                h = 0xcbf29ce484222325L;
                for (int z = info.minLevel; z <= info.maxLevel; z++) for (int y = cy * 256; y < cy * 256 + 256; y++) for (int x = cx * 256; x < cx * 256 + 256; x++) {
                    // Looked up through the existing chunk only: getRoomAt would create meta chunks as a side effect.
                    RoomDef r = null, e = null;
                    if (cell.hasChunk((x - cx * 256) / 8, (y - cy * 256) / 8)) { IsoMetaChunk ch = cell.getChunk((x - cx * 256) / 8, (y - cy * 256) / 8); r = ch.getRoomAt(x, y, z); e = ch.getEmptyOutsideAt(x, y, z); }
                    mixLong(r == null ? -1 : r.id);
                    mixLong(e == null ? -1 : e.id);
                }
                sb.append("H ").append(cx).append(' ').append(cy).append(' ').append(Long.toHexString(h)).append('\n');
                out.print(sb);
            }
        });
    }
}
