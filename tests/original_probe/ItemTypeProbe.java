import zombie.scripting.objects.ItemType;
public class ItemTypeProbe { public static void main(String[] args) throws Exception { for (var f : ItemType.class.getFields()) if (f.getType() == ItemType.class) System.out.println(f.getName()+"="+f.get(null)); } }
