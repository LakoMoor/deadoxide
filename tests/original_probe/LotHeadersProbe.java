import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.ArrayList;
import zombie.ZomboidFileSystem;
import zombie.iso.BuildingDef;
import zombie.iso.IsoLot;
import zombie.iso.IsoMetaCell;
import zombie.iso.IsoMetaGrid;
import zombie.iso.IsoWorld;
import zombie.iso.LotHeader;
import zombie.iso.MapFiles;
import zombie.iso.MetaObject;
import zombie.iso.RoomDef;

// Loads every .lotheader of one map folder through the original IsoMetaGrid.MetaGridLoaderThread.loadCell
// (single thread, cells in x-then-y order, no zombie voronois so intensities are the file's own) and dumps
// headers, rooms and buildings. Run from the game root: LotHeadersProbe "Muldraugh, KY"
public class LotHeadersProbe {
    static Object get(Object o, String name) throws Exception {
        Field x = o.getClass().getDeclaredField(name);
        x.setAccessible(true);
        return x.get(o);
    }
    static void set(Object o, String name, Object v) throws Exception {
        Field x = o.getClass().getDeclaredField(name);
        x.setAccessible(true);
        x.set(o, v);
    }

    public static void main(String[] args) throws Exception {
        java.io.PrintStream out = new java.io.PrintStream(new java.io.BufferedOutputStream(new java.io.FileOutputStream(java.io.FileDescriptor.out), 1 << 20), false, "UTF-8");
        System.setOut(System.err);
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-lotheaders-probe");
        ZomboidFileSystem.instance.init();
        zombie.network.GameServer.server = true;
        zombie.core.random.RandStandard.INSTANCE.init();
        String dir = args[0];
        String path = ZomboidFileSystem.instance.getDirectoryString("media/maps/" + dir + "/");
        java.io.File file = new java.io.File(path);
        MapFiles mf = new MapFiles(dir, path, file.getAbsolutePath(), 0);
        IsoLot.MapFiles.add(mf);
        if (!mf.load()) throw new RuntimeException("MapFiles.load false");
        mf.postLoad();

        IsoWorld.instance.setZombieVoronois(new ArrayList<>());
        IsoMetaGrid grid = IsoWorld.instance.getMetaGrid();
        grid.minX = mf.minX; grid.minY = mf.minY; grid.maxX = mf.maxX; grid.maxY = mf.maxY;
        set(grid, "grid", new IsoMetaCell[mf.maxX - mf.minX + 1][mf.maxY - mf.minY + 1]);
        set(grid, "width", mf.maxX - mf.minX + 1);
        set(grid, "height", mf.maxY - mf.minY + 1);
        Class<?> tc = Class.forName("zombie.iso.IsoMetaGrid$MetaGridLoaderThread");
        Constructor<?> ctor = tc.getDeclaredConstructor(IsoMetaGrid.class, int.class);
        ctor.setAccessible(true);
        Object thread = ctor.newInstance(grid, mf.minY);
        Method loadCell = tc.getDeclaredMethod("loadCell", MapFiles.class, int.class, int.class);
        loadCell.setAccessible(true);

        out.println("M " + mf.minX + " " + mf.minY + " " + mf.maxX + " " + mf.maxY + " " + mf.minCell300X + " " + mf.minCell300Y + " " + mf.maxCell300X + " " + mf.maxCell300Y);
        StringBuilder sb = new StringBuilder();
        for (int y = mf.minY; y <= mf.maxY; y++) { for (int x = mf.minX; x <= mf.maxX; x++) sb.append(mf.hasCell(x, y) ? '1' : '0'); sb.append('\n'); }
        for (int y = mf.minCell300Y; y <= mf.maxCell300Y; y++) { for (int x = mf.minCell300X; x <= mf.maxCell300X; x++) sb.append(mf.hasCell300(x, y) ? '1' : '0'); sb.append('\n'); }
        out.print(sb);
        Field rx = RoomDef.class.getDeclaredField("x");
        Field mtype = MetaObject.class.getDeclaredField("type"); mtype.setAccessible(true);
        for (int x = mf.minX; x <= mf.maxX; x++) {
            for (int y = mf.minY; y <= mf.maxY; y++) {
                if (!mf.hasCell(x, y)) continue;
                loadCell.invoke(thread, mf, x, y);
                LotHeader h = mf.getLotHeader(x, y);
                IsoMetaCell c = grid.getCellData(x, y);
                sb.setLength(0);
                sb.append("C ").append(x).append(' ').append(y).append(' ').append(h.version).append(' ').append(h.width).append(' ').append(h.height).append(' ')
                  .append(h.minLevel).append(' ').append(h.maxLevel).append(' ').append(h.fixed2x).append(' ').append(h.tilesUsed.size()).append(' ').append(c.roomList.size())
                  .append(' ').append(c.buildings.size()).append('\n');
                for (String t : h.tilesUsed) sb.append("T ").append(t).append('\n');
                for (RoomDef r : c.roomList) {
                    sb.append("R ").append(r.id).append(' ').append(r.metaId).append(' ').append(r.level).append(' ').append(r.x).append(' ').append(r.y).append(' ').append(r.x2)
                      .append(' ').append(r.y2).append(' ').append(r.area).append(' ').append(r.building == null ? -1 : r.building.id).append(" [").append(r.name).append("]");
                    for (RoomDef.RoomRect q : r.rects) sb.append(' ').append(q.x).append(',').append(q.y).append(',').append(q.w).append(',').append(q.h);
                    sb.append(" |");
                    for (MetaObject o : r.objects) sb.append(' ').append(mtype.getInt(o)).append(',').append(o.getX()).append(',').append(o.getY());
                    sb.append('\n');
                }
                for (BuildingDef b : c.buildings) {
                    sb.append("B ").append(b.id).append(' ').append(b.metaId).append(' ').append(b.x).append(' ').append(b.y).append(' ').append(b.x2).append(' ').append(b.y2).append(" r");
                    for (RoomDef r : b.rooms) sb.append(' ').append(r.id);
                    sb.append(" e");
                    for (RoomDef r : b.emptyoutside) sb.append(' ').append(r.id);
                    sb.append(" c");
                    gnu.trove.list.array.TShortArrayList oc = (gnu.trove.list.array.TShortArrayList) get(b, "overlappedChunks");
                    for (int i = 0; i < oc.size(); i++) sb.append(' ').append(oc.get(i));
                    sb.append('\n');
                }
                sb.append("Z ");
                for (int i = 0; i < 1024; i++) { int v = h.getZombieIntensity(i) & 255; sb.append(Character.forDigit(v >> 4, 16)).append(Character.forDigit(v & 15, 16)); }
                sb.append('\n');
                out.print(sb);
            }
        }
        out.flush();
    }
}
