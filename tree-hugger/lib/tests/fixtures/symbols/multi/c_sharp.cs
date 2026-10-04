// Multi-file fixture: C#.

namespace Multi
{
    public class Ledger
    {
        private int total;

        public int Total { get; set; }

        public void Add(int amount)
        {
            total += amount;
        }

        public static Ledger Empty()
        {
            return new Ledger();
        }
    }

    public interface IAuditable
    {
        void Audit();
    }
}
