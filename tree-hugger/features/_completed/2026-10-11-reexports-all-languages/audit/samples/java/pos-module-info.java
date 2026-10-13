module com.example.app {
    requires transitive java.sql;
    requires static transitive java.logging;
    requires java.base;
    exports com.example.api;
    exports com.example.internal to com.example.friend, com.example.other;
    opens com.example.model;
    opens com.example.model2 to com.example.json;
    uses com.example.Svc;
    provides com.example.Svc with com.example.SvcImpl;
}
