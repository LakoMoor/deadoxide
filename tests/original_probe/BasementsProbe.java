import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.Arrays;
import zombie.basements.Basements;
import zombie.iso.*;

// Inside the booted original: the world's OWN meta grid, i.e. after zombie.basements.Basements merged the procedural
// basements chosen for the fixed seed. Prints the placements, the global building list, and for a cell range the
// room and building lists, the rooms of every meta chunk (ids sorted: their order depends on an identity-hash walk in
// the original) and a hash of room lookups over all squares and levels -32..8.
// Args: <scratch dir> x1 y1 x2 y2
public class BasementsProbe {
    static long h;
    static void mix(int v) { for (int i = 0; i < 4; i++) { h ^= (v >>> (8 * i)) & 255; h *= 0x100000001b3L; } }
    static void mixLong(long v) { mix((int) v); mix((int) (v >> 32)); }

    public static void main(String[] a) throws Exception {
        int x1 = Integer.parseInt(a[1]), y1 = Integer.parseInt(a[2]), x2 = Integer.parseInt(a[3]), y2 = Integer.parseInt(a[4]);
        ServerHarness.run(a[0], out -> {
            IsoMetaGrid g = IsoWorld.instance.getMetaGrid();
            StringBuilder sb = new StringBuilder();
            sb.append("SEED ").append(zombie.iso.worldgen.WorldGenParams.INSTANCE.getSeed()).append('\n');
            Field pf = Basements.class.getDeclaredField("basementPlacements"); pf.setAccessible(true);
            for (Object p : (ArrayList<?>) pf.get(Basements.getInstance())) {
                Class<?> c = p.getClass();
                sb.append("P");
                for (String f : new String[]{"name", "x", "y", "z", "w", "h"}) { Field ff = c.getDeclaredField(f); ff.setAccessible(true); sb.append(' ').append(ff.get(p)); }
                sb.append('\n');
            }
            Field bf = IsoMetaGrid.class.getDeclaredField("buildings"); bf.setAccessible(true);
            ArrayList<BuildingDef> buildings = (ArrayList<BuildingDef>) bf.get(g);
            h = 0xcbf29ce484222325L;
            for (BuildingDef b : buildings) mixLong(b.id);
            sb.append("B ").append(buildings.size()).append(' ').append(Long.toHexString(h)).append('\n');
            Field rf = IsoMetaChunk.class.getDeclaredField("rooms"); rf.setAccessible(true);
            Field rs = IsoMetaChunk.class.getDeclaredField("roomsSize"); rs.setAccessible(true);
            for (int cx = x1; cx <= x2; cx++) for (int cy = y1; cy <= y2; cy++) {
                IsoMetaCell cell = g.getCellData(cx, cy);
                if (cell == null || IsoLot.getHeader(cx, cy) == null) continue;
                sb.append("C ").append(cx).append(' ').append(cy).append(' ').append(cell.roomList.size()).append(' ').append(cell.buildings.size()).append('\n');
                for (RoomDef r : cell.roomList) sb.append("R ").append(r.id).append(' ').append(r.metaId).append(' ').append(r.level).append(' ').append(r.x).append(' ').append(r.y).append(' ').append(r.x2).append(' ').append(r.y2).append(' ').append(r.building == null ? -1 : r.building.id).append(" [").append(r.name).append("]\n");
                for (BuildingDef b : cell.buildings) sb.append("D ").append(b.id).append(' ').append(b.metaId).append(' ').append(b.x).append(' ').append(b.y).append(' ').append(b.x2).append(' ').append(b.y2).append(' ').append(b.rooms.size()).append(' ').append(b.emptyoutside.size()).append('\n');
                for (int ly = 0; ly < 32; ly++) for (int lx = 0; lx < 32; lx++) {
                    if (!cell.hasChunk(lx, ly)) continue;
                    IsoMetaChunk ch = cell.getChunk(lx, ly);
                    int n = rs.getInt(ch);
                    if (n == 0) continue;
                    RoomDef[] rooms = (RoomDef[]) rf.get(ch);
                    long[] ids = new long[n];
                    for (int i = 0; i < n; i++) ids[i] = rooms[i].id;
                    Arrays.sort(ids);
                    sb.append("K ").append(cx).append(' ').append(cy).append(' ').append(lx).append(' ').append(ly);
                    for (long id : ids) sb.append(' ').append(id);
                    sb.append('\n');
                }
                h = 0xcbf29ce484222325L;
                for (int z = -32; z <= 8; z++) for (int y = cy * 256; y < cy * 256 + 256; y++) for (int x = cx * 256; x < cx * 256 + 256; x++) {
                    RoomDef r = null, e = null;
                    if (cell.hasChunk((x - cx * 256) / 8, (y - cy * 256) / 8)) { IsoMetaChunk ch = cell.getChunk((x - cx * 256) / 8, (y - cy * 256) / 8); r = ch.getRoomAt(x, y, z); e = ch.getEmptyOutsideAt(x, y, z); }
                    mixLong(r == null ? -1 : r.id);
                    mixLong(e == null ? -1 : e.id);
                }
                sb.append("H ").append(cx).append(' ').append(cy).append(' ').append(Long.toHexString(h)).append('\n');
            }
            out.print(sb);
        });
    }
}
