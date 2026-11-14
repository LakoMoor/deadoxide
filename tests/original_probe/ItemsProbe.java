import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import zombie.scripting.ScriptManager;
import zombie.scripting.objects.Item;

// Inside the booted original: every item script, sorted by full name, with the values of the fields named in the given
// list (one per line; produced by tools/gen_item_params.py). Floats as raw bits, null as the text "null".
// Args: <scratch dir> <field list file>
public class ItemsProbe {
    public static void main(String[] a) throws Exception {
        List<String> names = Files.readAllLines(Paths.get(a[1]));
        ServerHarness.run(a[0], out -> {
            ArrayList<Field> fields = new ArrayList<>();
            for (String n : names) { if (n.isEmpty()) continue; Field f = Item.class.getDeclaredField(n); f.setAccessible(true); fields.add(f); }
            ArrayList<Item> items = new ArrayList<>(ScriptManager.instance.getAllItems());
            items.sort((x, y) -> x.getFullName().compareTo(y.getFullName()));
            StringBuilder sb = new StringBuilder();
            for (Item it : items) {
                sb.append(it.getFullName());
                for (Field f : fields) {
                    Object v = f.get(it);
                    sb.append('\t');
                    if (v instanceof Float fl) sb.append(Integer.toHexString(Float.floatToIntBits(fl))); else sb.append(v);
                }
                sb.append('\n');
            }
            out.print(sb);
        });
    }
}
