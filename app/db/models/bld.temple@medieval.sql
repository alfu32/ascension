-- bld.temple@medieval: Medieval temple (mesh.temple_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.temple_medieval','Medieval temple');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.temple_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.dressed_stone',0,'','{"w":3.7,"d":3.7,"h":0.5,"steps":3,"inset":0.1}'),
  ('nave','box','mat.dressed_stone',0,'','{"y":0.5,"z":-0.35,"yaw":90,"w":2.7,"d":2.3,"h":2.2,"gable":55,"side_windows":0,"windows":3,"window_y":0.75,"window_w":0.16,"window_h":0.75}'),
  ('buttresses','column_row','mat.dressed_stone',0,'','{"x":-1.2,"y":0.5,"z":-0.35,"yaw":90,"count":4,"radius":0.12,"spacing":0.7,"h":2.05,"segments":4,"capital":0,"tint":"#d4cec4","repeat":2,"dx":2.4}'),
  ('nave_roof','gable_roof','mat.slate',0,'','{"y":2.7,"z":-0.35,"yaw":90,"w":2.7,"d":2.3,"pitch":52,"overhang":0.12,"t":0.1}'),
  ('nave_roof_cap','gable_roof','mat.cloth',1,'','{"y":3.822,"z":-0.35,"yaw":90,"w":2.98,"d":0.8,"pitch":52,"overhang":0,"t":0.03}'),
  ('tower_w','box','mat.dressed_stone',0,'','{"x":-1.05,"y":0.5,"z":1.15,"w":0.85,"d":0.85,"h":3.3,"side_windows":1,"window_y":2.1,"window_w":0.12,"window_h":0.5}'),
  ('tower_e','box','mat.dressed_stone',0,'','{"x":1.05,"y":0.5,"z":1.15,"w":0.85,"d":0.85,"h":3.3,"side_windows":1,"window_y":2.1,"window_w":0.12,"window_h":0.5}'),
  ('spires','hip_roof','mat.slate',0,'','{"x":-1.05,"y":3.8,"z":1.15,"w":0.85,"d":0.85,"pitch":74,"overhang":0.06,"t":0.06,"repeat":2,"dx":2.1}'),
  ('spire_tips','hip_roof','mat.cloth',1,'','{"x":-1.05,"y":4.71,"z":1.15,"w":0.34,"d":0.34,"pitch":74,"overhang":0,"t":0.03,"repeat":2,"dx":2.1}'),
  ('front','box','mat.dressed_stone',0,'','{"y":0.5,"z":1,"w":1.26,"d":0.3,"h":2.6,"gable":55,"tint":"#e4ded4"}'),
  ('portal','arch','mat.dressed_stone',0,'','{"y":0.5,"z":1.17,"w":1.1,"h":1.5,"t":0.06,"span":0.6,"spring":0.8,"tint":"#cfc8bc"}'),
  ('curtain','box','mat.cloth',1,'','{"y":0.5,"z":1.15,"w":0.6,"d":0.02,"h":1}'),
  ('rose','shield','mat.cloth',1,'','{"y":2.55,"z":1.16,"w":0.62,"t":0.04,"boss":0.12,"segments":10,"rim":"#d4cec4"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.temple_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.temple'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.temple_medieval'), (SELECT id FROM def WHERE key='mat.dressed_stone'), 100);
