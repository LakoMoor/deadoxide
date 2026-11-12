import java.nio.*;
import java.nio.file.*;
import java.util.*;
import zombie.GameWindow;
import zombie.iso.zones.Zone;
import zombie.util.SharedStrings;
public class FishingZoneSaveProbe {
 public static void main(String[] args)throws Exception {
  ByteBuffer buf=ByteBuffer.wrap(Files.readAllBytes(Path.of(args[0])));
  byte[] magic=new byte[4];buf.get(magic);int version=buf.getInt();int n=buf.getInt();
  HashMap<Integer,String> strings=new HashMap<>();for(int i=0;i<n;i++)strings.put(i,GameWindow.ReadString(buf));
  int count=buf.getInt(), fishing=0;SharedStrings shared=new SharedStrings();
  System.out.printf("ZONEFILE version=%d strings=%d zones=%d%n",version,n,count);
  for(int i=0;i<count;i++){
   Zone z=new Zone().load(buf,version,strings,shared);
   if("Fishing".equals(z.type)){fishing++;System.out.printf("FZONE %s,%d,%d,%d,%d,%d,%s,%d%n",z.name,z.x,z.y,z.z,z.w,z.h,z.geometryType,z.lastActionTimestamp);}
  }
  System.out.printf("ZONEFILE fishing=%d position=%d%n",fishing,buf.position());
 }
}
