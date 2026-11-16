-- unit.biplane@industrial: Biplane (mesh.biplane_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.biplane_industrial','Biplane');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.biplane_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.canvas',0,'','{"y":2.7,"z":-0.05,"w":0.3,"d":1.75,"h":0.3,"taper":0.25,"bank":14,"tint":"#c9c0a4"}'),
  ('cowling','cylinder','mat.cast_iron',0,'fuselage','{"y":0.15,"z":0.86,"w":0.36,"h":0.18,"segments":8,"pitch":90}'),
  ('propeller','propeller','mat.timber',0,'fuselage','{"y":0.15,"z":1.06,"w":0.9,"blades":2,"chord":0.09}'),
  ('wing_low','box','mat.canvas',0,'fuselage','{"y":-0.02,"z":0.35,"w":2.5,"d":0.42,"h":0.04,"tint":"#d2c9ac"}'),
  ('wing_top','box','mat.canvas',0,'fuselage','{"y":0.52,"z":0.42,"w":2.8,"d":0.44,"h":0.045,"tint":"#d2c9ac"}'),
  ('wing_tips','box','mat.cloth',1,'fuselage','{"x":-1.18,"y":0.568,"z":0.42,"w":0.44,"d":0.445,"h":0.01,"repeat":2,"dx":2.36}'),
  ('struts','box','mat.timber',0,'fuselage','{"x":-0.95,"y":0,"z":0.35,"w":0.03,"d":0.06,"h":0.54,"repeat":2,"dx":1.9}'),
  ('struts_back','box','mat.timber',0,'fuselage','{"x":-0.95,"y":0,"z":0.52,"w":0.03,"d":0.06,"h":0.54,"repeat":2,"dx":1.9,"far":0}'),
  ('cabane','box','mat.timber',0,'fuselage','{"x":-0.12,"y":0.28,"z":0.42,"w":0.03,"d":0.04,"h":0.26,"repeat":2,"dx":0.24,"far":0}'),
  ('pilot','dome','mat.leather',0,'fuselage','{"y":0.28,"z":-0.05,"w":0.17,"h":0.14,"segments":6}'),
  ('gun_w','cylinder','mat.cast_iron',0,'fuselage','{"x":-0.07,"y":0.3,"z":0.3,"w":0.04,"h":0.55,"segments":4,"pitch":90}'),
  ('spark_w','cone','mat.flash',0,'gun_w','{"y":0.55,"w":0.07,"h":0.07,"segments":3,"flash":4,"far":0}'),
  ('flash_w','cone','mat.flash',0,'spark_w','{"y":0.24,"w":0.2,"h":0.24,"segments":5,"pitch":180,"far":0}'),
  ('gun_e','cylinder','mat.cast_iron',0,'fuselage','{"x":0.07,"y":0.3,"z":0.3,"w":0.04,"h":0.55,"segments":4,"pitch":90}'),
  ('spark_e','cone','mat.flash',0,'gun_e','{"y":0.55,"w":0.07,"h":0.07,"segments":3,"flash":4,"far":0}'),
  ('flash_e','cone','mat.flash',0,'spark_e','{"y":0.24,"w":0.2,"h":0.24,"segments":5,"pitch":180,"far":0}'),
  ('tailplane','box','mat.canvas',0,'fuselage','{"y":0.14,"z":-0.82,"w":0.95,"d":0.28,"h":0.03,"tint":"#d2c9ac"}'),
  ('fin','box','mat.cloth',1,'fuselage','{"y":0.15,"z":-0.82,"w":0.03,"d":0.3,"h":0.34,"taper":0.4}'),
  ('gear','box','mat.timber',0,'fuselage','{"x":-0.14,"y":-0.3,"z":0.4,"w":0.03,"d":0.04,"h":0.3,"repeat":2,"dx":0.28,"far":0}'),
  ('gear_wheels','cylinder','mat.rubber',0,'fuselage','{"x":-0.24,"y":-0.4,"z":0.4,"w":0.2,"h":0.05,"segments":8,"roll":90,"repeat":2,"dx":0.43,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.biplane_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.biplane'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.biplane_industrial'), (SELECT id FROM def WHERE key='mat.canvas'), 100);
